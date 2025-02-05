use std::sync::Arc;
use chrono::{DateTime, Utc};
use git2::{IndexAddOption, Oid, Repository};
use itertools::Itertools;
use tokio::sync::RwLock as ARwLock;
use tokio::time::Instant;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::custom_error::MapErrToString;
use crate::files_correction::{deserialize_path, get_active_workspace_folder, get_project_dirs, serialize_path};
use crate::global_context::GlobalContext;
use crate::git::{FileChange, FileChangeStatus};
use crate::git::operations::{checkout_head_and_branch_to_commit, commit, get_commit_datetime, get_diff_statuses, get_diff_statuses_index_to_commit, get_or_create_branch, stage_changes, DiffStatusType, open_or_init_repo};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Checkpoint {
    #[serde(serialize_with = "serialize_path", deserialize_with = "deserialize_path")]
    pub workspace_folder: PathBuf,
    pub commit_hash: String,
}

impl Checkpoint {
    pub fn workspace_hash(&self) -> String {
        official_text_hashing_function(&self.workspace_folder.to_string_lossy().to_string())
    }
}

pub async fn create_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>,
    prev_checkpoint: Option<&Checkpoint>,
    chat_id: &str,
) -> Result<(Checkpoint, Vec<FileChange>, Repository, Vec<Repository>), String> {
    let (cache_dir, vcs_roots) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.documents_state.workspace_vcs_roots.clone())
    };
    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
        .ok_or_else(|| "No active workspace folder".to_string())?;
    let nested_vcs_roots: Vec<PathBuf> = {
        let vcs_roots_locked = vcs_roots.lock().unwrap();
        vcs_roots_locked.iter()
            .filter(|vcs| vcs.starts_with(&workspace_folder) && **vcs != workspace_folder).cloned().collect()
    };
    let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());

    if let Some(prev_checkpoint) = prev_checkpoint {
        if prev_checkpoint.workspace_hash() != workspace_folder_hash {
            return Err("Can not create checkpoint for different workspace folder".to_string());
        }
    }

    let t0 = Instant::now();

    let shadow_repo_path  = cache_dir.join("shadow_git").join(&workspace_folder_hash);
    let repo = Repository::open(&shadow_repo_path).map_err_with_prefix("Failed to open repo:")?;
    repo.set_workdir(&workspace_folder, false).map_err_with_prefix("Failed to set workdir:")?;
    let repo_workdir = repo.workdir().ok_or("Failed to get workdir just set.".to_string())?;
    let mut nested_repos = Vec::new();
    for vcs_root in nested_vcs_roots {
        let vcs_root_hash = official_text_hashing_function(&vcs_root.to_string_lossy().to_string());
        let vcs_root_git_path = cache_dir.join("shadow_git").join("nested").join(&vcs_root_hash);
        let nested_repo = open_or_init_repo(&vcs_root_git_path).map_err_with_prefix("Failed to open nested repo:")?;
        nested_repo.set_workdir(&vcs_root, false).map_err_with_prefix("Failed to set nested workdir:")?;
        nested_repos.push(nested_repo);
    }

    let has_commits = repo.head().map(|head| head.target().is_some()).unwrap_or(false);
    if !has_commits {
        return Err("No commits in shadow git repo, most likely initialization failed.".to_string());
    }

    let (checkpoint, file_changes) = {
        let branch = get_or_create_branch(&repo, &format!("refact-{chat_id}"))?;
        // if repo.head().map_err_with_prefix("Failed to get HEAD:")?.name() != branch.get().name() {
        //     let branch_commit = branch.get().peel_to_commit()
        //         .map_err_with_prefix("Failed to get branch commit:")?;
        //     repo.reset(branch_commit.as_object(), git2::ResetType::Mixed, None)
        //         .map_err_with_prefix("Failed to reset index:")?;
        //     repo.set_head(branch.get().name().ok_or("Branch name is not valid UTF-8")?)
        //         .map_err_with_prefix("Failed to set HEAD to branch:")?;
        // }

        let mut file_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, &repo, true)?;

        let mut nested_file_changes = Vec::new();
        for nested_repo in &nested_repos {
            let nested_repo_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, nested_repo, true)?;
            let nested_repo_workdir = nested_repo.workdir()
                .ok_or("Failed to get nested repo workdir".to_string())?;
            let nested_repo_rel_path = nested_repo_workdir.strip_prefix(repo_workdir).map_err_to_string()?;
            for change in &nested_repo_changes {
                file_changes.push(FileChange {
                    relative_path: nested_repo_rel_path.join(&change.relative_path),
                    absolute_path: change.absolute_path.clone(),
                    status: change.status.clone(),
                });
            }
            nested_file_changes.push((nested_repo, nested_repo_changes));
        }

        stage_changes(&repo, &file_changes)?;
        let commit_oid = commit(&repo, &branch, &format!("Auto commit for chat {chat_id}"), "Refact Agent", "agent@refact.ai")?;
        
        for (nested_repo, changes) in nested_file_changes {
            stage_changes(&nested_repo, &changes)?;
        }

        (Checkpoint {workspace_folder, commit_hash: commit_oid.to_string()}, vec![])
    };

    tracing::info!("Checkpoint created in {:.2}s", t0.elapsed().as_secs_f64());

    Ok((checkpoint, file_changes, repo, nested_repos))
}

pub async fn restore_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>, checkpoint_to_restore: &Checkpoint, chat_id: &str
) -> Result<(Checkpoint, Vec<FileChange>, DateTime<Utc>), String> {
    
    let (checkpoint_for_undo, _, repo, nested_repos) = 
        create_workspace_checkpoint(gcx.clone(), Some(checkpoint_to_restore), chat_id).await?;

    let commit_to_restore_oid = Oid::from_str(&checkpoint_to_restore.commit_hash).map_err_to_string()?;
    let reverted_to = get_commit_datetime(&repo, &commit_to_restore_oid)?;

    let mut files_changed = get_diff_statuses_index_to_commit(&repo, true, &commit_to_restore_oid)?;

    // let repo_workdir = repo.workdir().ok_or("Failed to get workdir.".to_string())?;
    // for nested_repo in &nested_repos {
    //     let nested_repo_workdir = nested_repo.workdir()
    //             .ok_or("Failed to get nested repo workdir".to_string())?;
    //     let nested_repo_rel_path = nested_repo_workdir.strip_prefix(repo_workdir).map_err_to_string()?;
    //     let mut nested_files_changed = get_diff_statuses(DiffStatusType::WorkdirToIndex, repository, include_untracked)
    // }

    // Invert status since we got changes in reverse order so that if it fails it does not update the workspace
    for change in &mut files_changed {
        change.status = match change.status {
            FileChangeStatus::ADDED => FileChangeStatus::DELETED,
            FileChangeStatus::DELETED => FileChangeStatus::ADDED,
            FileChangeStatus::MODIFIED => FileChangeStatus::MODIFIED,
        };
    }

    checkout_head_and_branch_to_commit(&repo, &format!("refact-{chat_id}"), &commit_to_restore_oid)?;

    for nested_repo in &nested_repos {
        let reset_index_result = (|| {
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), IndexAddOption::DEFAULT, Some(&mut |path, _matched_spec| {
                if path.join(".git").exists() { 1 } else { 0 }
            }))?;
            index.write()
        })();
        if let Err(e) = reset_index_result {
            let workdir = nested_repo.workdir().unwrap_or(&PathBuf::new()).to_string_lossy().to_string();
            tracing::error!("Failed to reset index for {workdir}: {e}");
            continue;
        }
    }

    Ok((checkpoint_for_undo, files_changed, reverted_to))
}

pub async fn initialize_shadow_git_repositories_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> () {
    let workspace_folders = get_project_dirs(gcx.clone()).await;
    let (cache_dir, workspace_vcs_roots_arc) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.documents_state.workspace_vcs_roots.clone())
    };
    let workspace_vcs_roots = workspace_vcs_roots_arc.lock().unwrap().clone();

    for workspace_folder in workspace_folders {
        let workspace_folder_str = workspace_folder.to_string_lossy().to_string();
        let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());
        let shadow_git_dir_path = cache_dir.join("shadow_git").join(&workspace_folder_hash);
        let nested_vcs_roots: Vec<PathBuf> = workspace_vcs_roots.iter()
            .filter(|r| r.starts_with(&workspace_folder) && **r != workspace_folder).cloned().collect();

        let repo = match open_or_init_repo(&shadow_git_dir_path) {
            Ok(repo) => repo,
            Err(e) => {
                tracing::error!("Failed to open or init repo for {workspace_folder_str}: {e}");
                continue;
            }
        };
        if let Err(e) = repo.set_workdir(&workspace_folder, false) {
            tracing::error!("Failed to set workdir for {workspace_folder_str}: {e}");
            continue;
        }

        let mut nested_repos = Vec::new();
        for nested_vcs in nested_vcs_roots {
            let nested_vcs_hash = official_text_hashing_function(&nested_vcs.to_string_lossy().to_string());
            let nested_vcs_git_path = cache_dir.join("shadow_git").join("nested").join(&nested_vcs_hash);
            let nested_repo = match open_or_init_repo(&nested_vcs_git_path) {
                Ok(repo) => repo,
                Err(e) => {
                    tracing::error!("Failed to open or init repo for {}: {}", nested_vcs.to_string_lossy(), e);
                    continue;
                },
            };
            if let Err(e) = nested_repo.set_workdir(&nested_vcs, false) {
                tracing::error!("Failed to set workdir for {}: {}", nested_vcs.to_string_lossy(), e);
                continue;
            }
            nested_repos.push(nested_repo);
        }

        let has_commits = repo.head().map(|head| head.target().is_some()).unwrap_or(false);
        if has_commits {
            tracing::info!("Shadow git repo for {} is already initialized.", workspace_folder_str);
            continue;
        }

        let t0 = Instant::now();
        let repo_workdir = repo.workdir().unwrap_or(&workspace_folder);

        let initial_commit_result: Result<Oid, String> = (|| {
            let mut file_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, &repo, true)?;
            let mut nested_file_changes = Vec::new();
            for nested_repo in &nested_repos {
                let nested_repo_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, nested_repo, true)?;
                let nested_repo_workdir = nested_repo.workdir()
                    .ok_or("Failed to get nested repo workdir".to_string())?;
                let nested_repo_rel_path = nested_repo_workdir.strip_prefix(repo_workdir).map_err_to_string()?;
                for change in &nested_repo_changes {
                    file_changes.push(FileChange {
                        relative_path: nested_repo_rel_path.join(&change.relative_path),
                        absolute_path: change.absolute_path.clone(),
                        status: change.status.clone(),
                    });
                }
                nested_file_changes.push((nested_repo, nested_repo_changes));
            }
            stage_changes(&repo, &file_changes)?;
            let mut index = repo.index().map_err_to_string()?;
            let tree_id = index.write_tree().map_err_to_string()?;
            let tree = repo.find_tree(tree_id).map_err_to_string()?;
            let signature = git2::Signature::now("Refact Agent", "agent@refact.ai").map_err_to_string()?;
            let commit = repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).map_err_to_string()?;
            for (nested_repo, changes) in nested_file_changes {
                stage_changes(&nested_repo, &changes)?;
            }
            Ok(commit)
        })();

        match initial_commit_result {
            Ok(_) => tracing::info!("Shadow git repo for {} initialized in {:.2}s.", workspace_folder_str, t0.elapsed().as_secs_f64()),
            Err(e) => {
                tracing::error!("Initial commit for {workspace_folder_str} failed: {e}");
                continue;
            }
        }
    }
}