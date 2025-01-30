use std::sync::{Arc, Mutex as StdMutex};
use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::RwLock as ARwLock;
use std::path::PathBuf;
use url::Url;
use serde::{Serialize, Deserialize};
use tracing::error;
use git2::{DiffOptions, Oid, Repository, Branch};

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::custom_error::MapErrToString;
use crate::files_correction::{get_active_workspace_folder, to_pathbuf_normalize};
use crate::global_context::GlobalContext;
use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::files_correction::{serialize_path, deserialize_path};

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitInfo {
    pub project_path: Url,
    pub commit_message: String,
    pub file_changes: Vec<FileChange>,
}
impl CommitInfo {
    pub fn get_project_name(&self) -> String {
        self.project_path.to_file_path().ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileChange {
    #[serde(serialize_with = "serialize_path", deserialize_with = "deserialize_path")]
    pub relative_path: PathBuf,
    #[serde(serialize_with = "serialize_path", deserialize_with = "deserialize_path")]
    pub absolute_path: PathBuf,
    pub status: FileChangeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileDiff {
    pub file_change: FileChange,
    pub content_before: String,
    pub content_after: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    ADDED,
    MODIFIED,
    DELETED,
}
impl FileChangeStatus {
    pub fn initial(&self) -> char {
        match self {
            FileChangeStatus::ADDED => 'A',
            FileChangeStatus::MODIFIED => 'M',
            FileChangeStatus::DELETED => 'D',
        }
    }
}

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

fn status_options(include_untracked: bool) -> git2::StatusOptions {
    let mut options = git2::StatusOptions::new();
    options
        .disable_pathspec_match(true)
        .include_ignored(false)
        .include_unmodified(false)
        .include_unreadable(false)
        .include_untracked(include_untracked)
        .recurse_ignored_dirs(false)
        .recurse_untracked_dirs(include_untracked)
        .rename_threshold(100)
        .update_index(true);
    options
}

pub fn git_ls_files(repository_path: &PathBuf) -> Option<Vec<PathBuf>> {
    let repository = Repository::open(repository_path)
        .map_err(|e| error!("Failed to open repository: {}", e)).ok()?;

    let statuses = repository.statuses(Some(&mut status_options(true)))
        .map_err(|e| error!("Failed to get statuses: {}", e)).ok()?;

    let mut files = Vec::new();
    for entry in statuses.iter() {
        let path = String::from_utf8_lossy(entry.path_bytes()).to_string();
        files.push(repository_path.join(path));
    }
    if !files.is_empty() { Some(files) } else { None }
}

pub fn get_or_create_branch<'repo>(repository: &'repo Repository, branch_name: &str) -> Result<Branch<'repo>, String> {
    match repository.find_branch(branch_name, git2::BranchType::Local) {
        Ok(branch) => Ok(branch),
        Err(_) => {
            let head_commit = repository.head()
                .and_then(|h| h.peel_to_commit())
                .map_err_with_prefix("Failed to get HEAD commit:")?;
            repository.branch(branch_name, &head_commit, false)
                .map_err_with_prefix("Failed to create branch:")
        }
    }
}

fn get_diff_statuses_worktree_to_head(repository: &Repository, include_untracked: bool) -> Result<Vec<FileChange>, String> {
    let repository_workdir = repository.workdir()
        .ok_or("Failed to get workdir from repository".to_string())?;
    
    let mut result = Vec::new();
    let statuses = repository.statuses(Some(&mut status_options(include_untracked)))
        .map_err_with_prefix("Failed to get statuses:")?;
    for entry in statuses.iter() {
        let status = entry.status();
        let relative_path = PathBuf::from(String::from_utf8_lossy(entry.path_bytes()).to_string());
        let absolute_path = to_pathbuf_normalize(&repository_workdir.join(&relative_path).to_string_lossy());

        if status.is_wt_new() || status.is_index_new() {
            result.push(FileChange {
                status: FileChangeStatus::ADDED,
                relative_path,
                absolute_path,
            });
        } else if status.is_wt_modified() || status.is_index_modified() || 
                    status.is_wt_typechange() || status.is_index_typechange() || 
                    status.is_conflicted() {
            result.push(FileChange {
                status: FileChangeStatus::MODIFIED,
                relative_path,
                absolute_path,
            });
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            result.push(FileChange {
                status: FileChangeStatus::DELETED,
                relative_path,
                absolute_path,
            });
        } else if status.is_ignored() || status.is_wt_renamed() || status.is_index_renamed() {
            tracing::error!("File status is {:?} for file {:?}, which should not be present due to status options.", status, relative_path);
        }
    }

    Ok(result)
}

pub fn get_diff_statuses_worktree_to_commit(repository: &Repository, include_untracked: bool, commit_oid: &git2::Oid) -> Result<Vec<FileChange>, String> {
    let head = repository.head().map_err_with_prefix("Failed to get HEAD:")?;
    let original_head_ref = head.is_branch().then(|| head.name().map(ToString::to_string)).flatten();
    let original_head_oid = head.target();

    repository.set_head_detached(commit_oid.clone()).map_err_with_prefix("Failed to set HEAD:")?;

    let result = get_diff_statuses_worktree_to_head(repository, include_untracked);

    let restore_result = match (&original_head_ref, original_head_oid) {
        (Some(head_ref), _) => repository.set_head(head_ref),
        (None, Some(oid)) => repository.set_head_detached(oid),
        (None, None) => Ok(()),
    };
    
    if let Err(restore_err) = restore_result {
        let prev_err = result.as_ref().err().cloned().unwrap_or_default();
        return Err(format!("{}\nFailed to restore head: {}", prev_err, restore_err));
    }

    result
}

pub fn stage_changes(repository: &Repository, file_changes: &Vec<FileChange>) -> Result<(), String> {
    let mut index = repository.index().map_err_with_prefix("Failed to get index:")?;

    for file_change in file_changes {
        match file_change.status {
            FileChangeStatus::ADDED | FileChangeStatus::MODIFIED => {
                index.add_path(&file_change.relative_path)
                    .map_err_with_prefix("Failed to add file to index:")?;
            },
            FileChangeStatus::DELETED => {
                index.remove_path(&file_change.relative_path)
                    .map_err_with_prefix("Failed to remove file from index:")?;
            },
        }
    }

    index.write().map_err_with_prefix("Failed to write index:")?;
    Ok(())
}

pub fn get_configured_author_email_and_name(repository: &Repository) -> Result<(String, String), String> {
    let config = repository.config()
        .map_err_with_prefix("Failed to get repository config:")?;
    let author_email = config.get_string("user.email")
        .map_err_with_prefix("Failed to get author email:")?;
    let author_name = config.get_string("user.name")
        .map_err_with_prefix("Failed to get author name:")?;
    Ok((author_email, author_name))
}

pub fn commit(repository: &Repository, branch: &Branch, message: &str, author_name: &str, author_email: &str) -> Result<Oid, String> {
    let mut index = repository.index().map_err_with_prefix("Failed to get index:")?;
    let tree_id = index.write_tree().map_err_with_prefix("Failed to write tree:")?;
    let tree = repository.find_tree(tree_id).map_err_with_prefix("Failed to find tree:")?;

    let signature = git2::Signature::now(author_name, author_email)
        .map_err_with_prefix("Failed to create signature:")?;
    let branch_ref_name = branch.get().name().ok_or("Invalid branch name".to_string())?;

    let parent_commit = if let Some(target) = branch.get().target() {
        repository.find_commit(target)
            .map_err(|e| format!("Failed to find branch commit: {}", e))?
    } else {
        return Err("No parent commits found".to_string());
    };

    let commit = repository.commit(
        Some(branch_ref_name), &signature, &signature, message, &tree, &[&parent_commit]
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    repository.set_head(branch_ref_name).map_err_with_prefix("Failed to set branch as head:")?;

    Ok(commit)
}

pub fn get_datetime_from_commit(repository: &Repository, commit_oid: &Oid) -> Result<DateTime<Utc>, String> {
    let commit = repository.find_commit(commit_oid.clone()).map_err_to_string()?;

    Utc.timestamp_opt(commit.time().seconds(), 0).single()
        .ok_or_else(|| "Failed to get commit datetime".to_string())
}

fn git_diff<'repo>(repository: &'repo Repository, file_changes: &Vec<FileChange>) -> Result<git2::Diff<'repo>, String> {
    let mut diff_options = DiffOptions::new();
    diff_options.include_untracked(true);
    diff_options.recurse_untracked_dirs(true);
    for file_change in file_changes {
        diff_options.pathspec(&file_change.relative_path);
    }

    let mut sorted_file_changes = file_changes.clone();
    sorted_file_changes.sort_by_key(|fc| {
        std::fs::metadata(&fc.relative_path).map(|meta| meta.len()).unwrap_or(0)
    });

    // Create a new temporary tree, with all changes staged
    let mut index = repository.index().map_err(|e| format!("Failed to get repository index: {}", e))?;
    for file_change in &sorted_file_changes {
        match file_change.status {
            FileChangeStatus::ADDED | FileChangeStatus::MODIFIED => {
                index.add_path(&file_change.relative_path)
                    .map_err(|e| format!("Failed to add file to index: {}", e))?;
            },
            FileChangeStatus::DELETED => {
                index.remove_path(&file_change.relative_path)
                    .map_err(|e| format!("Failed to remove file from index: {}", e))?;
            },
        }
    }
    let oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    let new_tree = repository.find_tree(oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    let head = repository.head().and_then(|head_ref| head_ref.peel_to_tree())
        .map_err(|e| format!("Failed to get HEAD tree: {}", e))?;

    let diff = repository.diff_tree_to_tree(Some(&head), Some(&new_tree), Some(&mut diff_options))
        .map_err(|e| format!("Failed to generate diff: {}", e))?;

    Ok(diff)
}

/// Similar to `git diff`, from specified file changes.
pub fn git_diff_as_string(repository: &Repository, file_changes: &Vec<FileChange>, max_size: usize) -> Result<String, String> {
    let diff = git_diff(repository, file_changes)?;

    let mut diff_str = String::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        let line_content = std::str::from_utf8(line.content()).unwrap_or("");
        if diff_str.len() + line_content.len() < max_size {
            diff_str.push(line.origin());
            diff_str.push_str(line_content);
            if diff_str.len() > max_size {
                diff_str.truncate(max_size - 4);
                diff_str.push_str("...\n");
            }
        }
        true
    }).map_err(|e| format!("Failed to print diff: {}", e))?;

    Ok(diff_str)
}

pub async fn get_commit_information_from_current_changes(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<CommitInfo>
{
    let mut commits = Vec::new();

    let workspace_vcs_roots: Arc<StdMutex<Vec<PathBuf>>> = {
        let cx_locked = gcx.write().await;
        cx_locked.documents_state.workspace_vcs_roots.clone()
    };

    let vcs_roots_locked = workspace_vcs_roots.lock().unwrap();
    tracing::info!("get_commit_information_from_current_changes() vcs_roots={:?}", vcs_roots_locked);
    for project_path in vcs_roots_locked.iter() {
        let repository = match git2::Repository::open(project_path) {
            Ok(repo) => repo,
            Err(e) => { tracing::warn!("{}", e); continue; }
        };

        let file_changes = match get_diff_statuses_worktree_to_head(&repository, true) {
            Ok(changes) if changes.is_empty() => { continue; }
            Ok(changes) => changes,
            Err(e) => { tracing::warn!("{}", e); continue; }
        };

        commits.push(CommitInfo {
            project_path: Url::from_file_path(project_path).ok().unwrap_or_else(|| Url::parse("file:///").unwrap()),
            commit_message: "".to_string(),
            file_changes,
        });
    }

    commits
}

pub async fn generate_commit_messages(gcx: Arc<ARwLock<GlobalContext>>, commits: Vec<CommitInfo>) -> Vec<CommitInfo> {
    const MAX_DIFF_SIZE: usize = 4096;
    let mut commits_with_messages = Vec::new();
    for commit in commits {
        let project_path = commit.project_path.to_file_path().ok().unwrap_or_default();

        let repository = match git2::Repository::open(&project_path) {
            Ok(repo) => repo,
            Err(e) => { error!("{}", e); continue; }
        };

        let diff = match git_diff_as_string(&repository, &commit.file_changes, MAX_DIFF_SIZE) {
            Ok(d) if d.is_empty() => { continue; }
            Ok(d) => d,
            Err(e) => { error!("{}", e); continue; }
        };

        let commit_msg = match generate_commit_message_by_diff(gcx.clone(), &diff, &None).await {
            Ok(msg) => msg,
            Err(e) => { error!("{}", e); continue; }
        };

        commits_with_messages.push(CommitInfo {
            project_path: commit.project_path,
            commit_message: commit_msg,
            file_changes: commit.file_changes,
        });
    }

    commits_with_messages
}

pub fn open_or_initialize_repo(workdir: &PathBuf, git_dir_path: &PathBuf) -> Result<Repository, String> {
    match git2::Repository::open(&git_dir_path) {
        Ok(repo) => {
            repo.set_workdir(&workdir, false).map_err_to_string()?;
            Ok(repo)
        },
        Err(not_found_err) if not_found_err.code() == git2::ErrorCode::NotFound => {
            let repo = git2::Repository::init(&git_dir_path).map_err_to_string()?;
            repo.set_workdir(&workdir, false).map_err_to_string()?;

            {
                let tree_id = {
                    let mut index = repo.index().map_err_to_string()?;
                    index.write_tree().map_err_to_string()?
                };
                let tree = repo.find_tree(tree_id).map_err_to_string()?;
                let signature = git2::Signature::now("Refact Agent", "agent@refact.ai")
                    .map_err_to_string()?;
                repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[])
                    .map_err_to_string()?;
            }

            Ok(repo)
        },
        Err(e) => Err(e.to_string()),
    }
}

pub fn checkout_head_and_branch_to_commit(repo: &Repository, branch_name: &str, commit_oid: &Oid) -> Result<(), String> {
    let commit = repo.find_commit(commit_oid.clone()).map_err_with_prefix("Failed to find commit:")?;

    let mut branch_ref = repo.find_branch(branch_name, git2::BranchType::Local)
        .map_err_with_prefix("Failed to get branch:")?.into_reference();
    branch_ref.set_target(commit.id(),"Restoring checkpoint")
        .map_err_with_prefix("Failed to update branch reference:")?;

    repo.set_head(&format!("refs/heads/{}", branch_name))
        .map_err_with_prefix("Failed to set HEAD:")?;

    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.force().update_index(true);
    repo.checkout_head(Some(&mut checkout_opts)).map_err_with_prefix("Failed to checkout HEAD:")?;

    Ok(())
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

    // let timestamp_ms_before = std::time::SystemTime::now()
    //     .duration_since(std::time::UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();

    let shadow_repo_path  = cache_dir.join("shadow_git").join(&workspace_folder_hash);
    let repo = open_or_initialize_repo(&workspace_folder, &shadow_repo_path)
        .map_err_with_prefix("Failed to open or init repo:")?;

    let (checkpoint, file_changes) = {
        let branch = get_or_create_branch(&repo, &format!("refact-{chat_id}"))?;
        let file_changes = get_diff_statuses_worktree_to_head(&repo, true)?;
        stage_changes(&repo, &file_changes)?;
        let commit_oid = commit(&repo, &branch, &format!("Auto commit for chat {chat_id}"), "Refact Agent", "agent@refact.ai")?;

        (Checkpoint {workspace_folder, commit_hash: commit_oid.to_string()}, file_changes)
    };

    // let timestamp_ms_after = std::time::SystemTime::now()
    //     .duration_since(std::time::UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();

    // tracing::info!("Creating checkpoint took: {}", (timestamp_ms_after - timestamp_ms_before) as f64 / 1000.0);

    Ok((checkpoint, file_changes, repo))
}

pub async fn restore_workspace_checkpoint(
    gcx: Arc<ARwLock<GlobalContext>>, checkpoint_to_restore: &Checkpoint, chat_id: &str
) -> Result<(Checkpoint, Vec<FileChange>, DateTime<Utc>), String> {
    
    let (checkpoint_for_undo, _, repo) = 
        create_workspace_checkpoint(gcx.clone(), Some(checkpoint_to_restore), chat_id).await?;
    
    let commit_to_restore_oid = Oid::from_str(&checkpoint_to_restore.commit_hash).map_err_to_string()?;
    let reverted_to = get_datetime_from_commit(&repo, &commit_to_restore_oid)?;
    
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