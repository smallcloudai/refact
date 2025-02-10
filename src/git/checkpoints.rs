use std::sync::Arc;
use chrono::{DateTime, Utc};
use git2::{IndexAddOption, Oid, Repository};
use tokio::sync::RwLock as ARwLock;
use tokio::time::Instant;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::custom_error::MapErrToString;
use crate::file_filter::BLACKLISTED_DIRS;
use crate::files_correction::{deserialize_path, get_active_workspace_folder, get_project_dirs, serialize_path};
use crate::global_context::GlobalContext;
use crate::git::{FileChange, FileChangeStatus, DiffStatusType};
use crate::git::operations::{checkout_head_and_branch_to_commit, commit, get_commit_datetime, get_diff_statuses, get_diff_statuses_index_to_commit, get_or_create_branch, stage_changes, open_or_init_repo};

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

async fn open_shadow_repo_and_nested_repos(
    gcx: Arc<ARwLock<GlobalContext>>, workspace_folder: &Path, allow_init_main_repo: bool,
) -> Result<(Repository, Vec<Repository>, String), String> {
    fn open_repos(paths: &[PathBuf], allow_init: bool, nested: bool, cache_dir: &Path) -> Result<Vec<Repository>, String> {
        let mut result = Vec::new();
        for path in paths {
            let path_hash = official_text_hashing_function(&path.to_string_lossy().to_string());
            let git_dir_path = if nested {
                cache_dir.join("shadow_git").join("nested").join(&path_hash)
            } else {
                cache_dir.join("shadow_git").join(&path_hash)
            };
            let nested_repo = if allow_init {
                open_or_init_repo(&git_dir_path).map_err_to_string()
            } else {
                Repository::open(&git_dir_path).map_err_to_string()
            }?;
            nested_repo.set_workdir(path, false).map_err_to_string()?;
            for blacklisted_dir in  BLACKLISTED_DIRS {
                if let Err(e) = nested_repo.add_ignore_rule(blacklisted_dir) {
                    tracing::warn!("Failed to add ignore rule for {blacklisted_dir}: {e}");
                }
            }
            result.push(nested_repo);
        }
        Ok(result)
    }
    
    let (cache_dir, vcs_roots) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.documents_state.workspace_vcs_roots.clone())
    };
    let nested_vcs_roots: Vec<PathBuf> = {
        let vcs_roots_locked = vcs_roots.lock().unwrap();
        vcs_roots_locked.iter()
            .filter(|vcs| vcs.starts_with(&workspace_folder) && **vcs != workspace_folder).cloned().collect()
    };
    let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());

    let repo = open_repos(&[workspace_folder.to_path_buf()], allow_init_main_repo, false, &cache_dir)?
        .into_iter().next().unwrap();
    let nested_repos = open_repos(&nested_vcs_roots, true, true, &cache_dir)?;

    Ok((repo, nested_repos, workspace_folder_hash))
}

fn get_file_changes_from_nested_repos<'a>(
    parent_repo: &'a Repository, nested_repos: &'a [Repository], include_abs_paths: bool
) -> Result<(Vec<(&'a Repository, Vec<FileChange>)>, Vec<FileChange>), String> {
    let repo_workdir = parent_repo.workdir().ok_or("Failed to get workdir.".to_string())?;
    let mut file_changes_per_repo = Vec::new();
    let mut file_changes_flatened = Vec::new();

    for nested_repo in nested_repos {
        let nested_repo_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, nested_repo, include_abs_paths)?;
        let nested_repo_workdir = nested_repo.workdir()
            .ok_or("Failed to get nested repo workdir".to_string())?;
        let nested_repo_rel_path = nested_repo_workdir.strip_prefix(repo_workdir).map_err_to_string()?;

        for change in &nested_repo_changes {
            file_changes_flatened.push(FileChange {
                relative_path: nested_repo_rel_path.join(&change.relative_path),
                absolute_path: change.absolute_path.clone(),
                status: change.status.clone(),
            });
        }
        file_changes_per_repo.push((nested_repo, nested_repo_changes));
    }

    Ok((file_changes_per_repo, file_changes_flatened))
}

pub async fn create_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>,
    prev_checkpoint: Option<&Checkpoint>,
    chat_id: &str,
) -> Result<(Checkpoint, Repository), String> {
    let t0 = Instant::now();

    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
        .ok_or_else(|| "No active workspace folder".to_string())?;
    let (repo, nested_repos, workspace_folder_hash) = 
        open_shadow_repo_and_nested_repos(gcx.clone(), &workspace_folder, false).await?;
    
    if let Some(prev_checkpoint) = prev_checkpoint {
        if prev_checkpoint.workspace_hash() != workspace_folder_hash {
            return Err("Can not create checkpoint for different workspace folder".to_string());
        }
    }

    let has_commits = repo.head().map(|head| head.target().is_some()).unwrap_or(false);
    if !has_commits {
        return Err("No commits in shadow git repo.".to_string());
    }

    let checkpoint = {
        let branch = get_or_create_branch(&repo, &format!("refact-{chat_id}"))?;

        let mut file_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, &repo, false)?;

        let (nested_file_changes, flatened_nested_file_changes) = 
            get_file_changes_from_nested_repos(&repo, &nested_repos, false)?;
        file_changes.extend(flatened_nested_file_changes);

        stage_changes(&repo, &file_changes)?;
        let commit_oid = commit(&repo, &branch, &format!("Auto commit for chat {chat_id}"), "Refact Agent", "agent@refact.ai")?;
        
        for (nested_repo, changes) in nested_file_changes {
            stage_changes(&nested_repo, &changes)?;
        }

        Checkpoint {workspace_folder, commit_hash: commit_oid.to_string()}
    };

    tracing::info!("Checkpoint created in {:.2}s", t0.elapsed().as_secs_f64());

    Ok((checkpoint, repo))
}

pub async fn preview_changes_for_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>, checkpoint_to_restore: &Checkpoint, chat_id: &str
) -> Result<(Vec<FileChange>, DateTime<Utc>, Checkpoint), String> {
    let (checkpoint_for_undo, repo) = create_workspace_checkpoint(gcx.clone(), Some(checkpoint_to_restore), chat_id).await?;

    let commit_to_restore_oid = Oid::from_str(&checkpoint_to_restore.commit_hash).map_err_to_string()?;
    let reverted_to = get_commit_datetime(&repo, &commit_to_restore_oid)?;

    let mut files_changed = get_diff_statuses_index_to_commit(&repo, &commit_to_restore_oid, true)?;

    // Invert status since we got changes in reverse order so that if it fails it does not update the workspace
    for change in &mut files_changed {
        change.status = match change.status {
            FileChangeStatus::ADDED => FileChangeStatus::DELETED,
            FileChangeStatus::DELETED => FileChangeStatus::ADDED,
            FileChangeStatus::MODIFIED => FileChangeStatus::MODIFIED,
        };
    }

    Ok((files_changed, reverted_to, checkpoint_for_undo))
}

pub async fn restore_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>, checkpoint_to_restore: &Checkpoint, chat_id: &str
) -> Result<(), String> {
    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
        .ok_or_else(|| "No active workspace folder".to_string())?;
    let (repo, nested_repos, workspace_folder_hash) = 
        open_shadow_repo_and_nested_repos(gcx.clone(), &workspace_folder, false).await?;
    if checkpoint_to_restore.workspace_hash() != workspace_folder_hash {
        return Err("Can not restore checkpoint for different workspace folder".to_string());
    }

    let commit_to_restore_oid = Oid::from_str(&checkpoint_to_restore.commit_hash).map_err_to_string()?;

    checkout_head_and_branch_to_commit(&repo, &format!("refact-{chat_id}"), &commit_to_restore_oid)?;
    
    for nested_repo in &nested_repos {
        let reset_index_result = nested_repo.index()
            .and_then(|mut index| {
                index.add_all(["*"], IndexAddOption::DEFAULT, Some(&mut |path, _| {
                    if path.as_os_str().as_encoded_bytes().last() == Some(&b'/') && path.join(".git").exists() {
                        1
                    } else {
                        0
                    }
                }))?;
                index.write()
            });
        if let Err(e) = reset_index_result {
            let workdir = nested_repo.workdir().unwrap_or(&PathBuf::new()).to_string_lossy().to_string();
            tracing::error!("Failed to reset index for {workdir}: {e}");
        }
    }

    Ok(())
}

pub async fn init_shadow_repos_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> () {
    let workspace_folders = get_project_dirs(gcx.clone()).await;

    for workspace_folder in workspace_folders {
        let workspace_folder_str = workspace_folder.to_string_lossy().to_string();

        let (repo, nested_repos) = match open_shadow_repo_and_nested_repos(gcx.clone(), &workspace_folder, true).await {
            Ok((repo, nested_repos, _)) => (repo, nested_repos),
            Err(e) => {
                tracing::error!("Failed to open or init shadow repo for {workspace_folder_str}: {e}");
                continue;
            }
        };

        let has_commits = repo.head().map(|head| head.target().is_some()).unwrap_or(false);
        if has_commits {
            tracing::info!("Shadow git repo for {} is already initialized.", workspace_folder_str);
            continue;
        }

        let t0 = Instant::now();

        let initial_commit_result: Result<Oid, String> = (|| {
            let mut file_changes = get_diff_statuses(DiffStatusType::WorkdirToIndex, &repo, false)?;
            let (nested_file_changes, all_nested_changes) = 
                get_file_changes_from_nested_repos(&repo, &nested_repos, false)?;
            file_changes.extend(all_nested_changes);

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