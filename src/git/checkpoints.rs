use std::sync::Arc;
use chrono::{DateTime, Utc};
use git2::{Repository, Oid};
use tokio::sync::RwLock as ARwLock;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::custom_error::MapErrToString;
use crate::files_correction::{deserialize_path, get_active_workspace_folder, get_project_dirs, serialize_path};
use crate::global_context::GlobalContext;
use crate::git::{FileChange, FileChangeStatus};
use crate::git::operations::{get_or_create_branch, get_diff_statuses_worktree_to_head, stage_changes, commit, 
                       get_diff_statuses_worktree_to_commit, checkout_head_and_branch_to_commit, get_commit_datetime};

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
) -> Result<(Checkpoint, Vec<FileChange>, Repository), String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
        .ok_or_else(|| "No active workspace folder".to_string())?;
    let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());

    if let Some(prev_checkpoint) = prev_checkpoint {
        if prev_checkpoint.workspace_hash() != workspace_folder_hash {
            return Err("Can not create checkpoint for different workspace folder".to_string());
        }
    }

    let shadow_repo_path  = cache_dir.join("shadow_git").join(&workspace_folder_hash);
    let repo = Repository::open(&shadow_repo_path).map_err_with_prefix("Failed to open repo:")?;
    repo.set_workdir(&workspace_folder, false).map_err_with_prefix("Failed to set workdir:")?;

    let (checkpoint, file_changes) = {
        let branch = get_or_create_branch(&repo, &format!("refact-{chat_id}"))?;
        // repo.set_head(branch.get().name().ok_or("Invalid branch name".to_string())?)
        //     .map_err_with_prefix("Failed to set head:")?;
        let file_changes = get_diff_statuses_worktree_to_head(&repo, true)?;
        stage_changes(&repo, &file_changes)?;
        let commit_oid = commit(&repo, &branch, &format!("Auto commit for chat {chat_id}"), "Refact Agent", "agent@refact.ai")?;

        (Checkpoint {workspace_folder, commit_hash: commit_oid.to_string()}, file_changes)
    };

    Ok((checkpoint, file_changes, repo))
}

pub async fn restore_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>, checkpoint_to_restore: &Checkpoint, chat_id: &str
) -> Result<(Checkpoint, Vec<FileChange>, DateTime<Utc>), String> {
    
    let (checkpoint_for_undo, _, repo) = 
        create_workspace_checkpoint(gcx.clone(), Some(checkpoint_to_restore), chat_id).await?;
    
    let commit_to_restore_oid = Oid::from_str(&checkpoint_to_restore.commit_hash).map_err_to_string()?;
    let reverted_to = get_commit_datetime(&repo, &commit_to_restore_oid)?;
    
    let mut files_changed = get_diff_statuses_worktree_to_commit(
            &repo, true, &commit_to_restore_oid)?;

    // Invert status since we got changes in reverse order so that if it fails it does not updates the workspace
    for change in &mut files_changed {
        change.status = match change.status {
            FileChangeStatus::ADDED => FileChangeStatus::DELETED,
            FileChangeStatus::DELETED => FileChangeStatus::ADDED,
            FileChangeStatus::MODIFIED => FileChangeStatus::MODIFIED,
        };
    }

    checkout_head_and_branch_to_commit(&repo, &format!("refact-{chat_id}"), &commit_to_restore_oid)?;

    Ok((checkpoint_for_undo, files_changed, reverted_to))
}

pub async fn initialize_shadow_git_repositories_if_needed(gcx: Arc<ARwLock<GlobalContext>>) -> () {
    let workspace_folders = get_project_dirs(gcx.clone()).await;
    let cache_dir = gcx.read().await.cache_dir.clone();

    for workspace_folder in workspace_folders {
        let workspace_folder_str = workspace_folder.to_string_lossy().to_string();
        let workspace_folder_hash = crate::ast::chunk_utils::official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());
        let shadow_git_dir_path = cache_dir.join("shadow_git").join(&workspace_folder_hash);
        
        match Repository::open(&shadow_git_dir_path) {
            Ok(_) => {
                tracing::info!("Shadow git repo for {workspace_folder_str} already exists and can be opened.");
                continue;
            },
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                tracing::info!("Shadow git repo for {workspace_folder_str} does not exist, creating one.");
            },
            Err(e) => {
                tracing::error!("Failed to open repository for {workspace_folder_str}: {e}");
                continue;
            },
        };

        match super::operations::clone_local_repo_without_checkout(&workspace_folder, &shadow_git_dir_path) {
            Ok(time_elapsed) => {
                tracing::info!("Shadow git repo for {workspace_folder_str} cloned successfully from original repo in {:.2}s", time_elapsed.as_secs_f64());
                continue;
            },
            Err(e) => {
                tracing::warn!("Failed to clone shadow git repo from {workspace_folder_str}, trying to create one.\nFail reason: {e}");
            }
        }

        let repo = match git2::Repository::init(&shadow_git_dir_path) {
            Ok(repo) => repo,
            Err(e) => {
                tracing::error!("Failed to initialize shadow git repo for {workspace_folder_str}: {e}");
                continue;
            },
        };
        if let Err(e) = repo.set_workdir(&workspace_folder, false) {
            tracing::error!("Failed to set workdir for {workspace_folder_str}: {e}");
            continue;
        }

        let initial_commit_result = (|| {
            let file_changes = get_diff_statuses_worktree_to_head(&repo, true)?;
            super::operations::stage_changes(&repo, &file_changes)?;
            let tree_id = repo.index().map_err_to_string()?.write_tree().map_err_to_string()?;
            let tree = repo.find_tree(tree_id).map_err_to_string()?;
            let signature = git2::Signature::now("Refact Agent", "agent@refact.ai").map_err_to_string()?;
            repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).map_err_to_string()
        })();

        match initial_commit_result {
            Ok(_) => tracing::info!("Shadow git repo for {workspace_folder_str} initialized."),
            Err(e) => {
                tracing::error!("Initial commit for {workspace_folder_str} failed: {e}");
                continue;
            }
        }
    }
}