use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::RwLock as ARwLock;
use std::path::{Path, PathBuf};
use url::Url;
use serde::{Serialize, Deserialize};
use tracing::error;
use git2::{Branch, DiffOptions, Oid, Repository, Signature, Status, StatusOptions};

use crate::ast::chunk_utils::official_text_hashing_function;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

pub fn git_ls_files(repository_path: &PathBuf) -> Option<Vec<PathBuf>> {
    let repository = Repository::open(repository_path)
        .map_err(|e| error!("Failed to open repository: {}", e)).ok()?;

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_unmodified(true)
        .exclude_submodules(false)
        .include_ignored(false)
        .recurse_ignored_dirs(false);

    let statuses = repository.statuses(Some(&mut status_options))
        .map_err(|e| error!("Failed to get statuses: {}", e)).ok()?;

    let mut files = Vec::new();
    for entry in statuses.iter() {
        let path = String::from_utf8_lossy(entry.path_bytes()).to_string();
        files.push(repository_path.join(path));
    }
    if !files.is_empty() { Some(files) } else { None }
}

/// Similar to git checkout -b <branch_name>
pub fn create_or_checkout_to_branch<'repo>(repository: &'repo Repository, branch_name: &str) -> Result<Branch<'repo>, String> {
    let branch = match repository.find_branch(branch_name, git2::BranchType::Local) {
        Ok(branch) => branch,
        Err(_) => {
            let head_commit = repository.head()
                .and_then(|h| h.peel_to_commit())
                .map_err(|e| format!("Failed to get HEAD commit: {}", e))?;
            repository.branch(branch_name, &head_commit, false)
                .map_err(|e| format!("Failed to create branch: {}", e))?
        }
    };

    // Checkout to the branch
    let object = repository.revparse_single(&("refs/heads/".to_owned() + branch_name))
        .map_err(|e| format!("Failed to revparse single: {}", e))?;
    repository.checkout_tree(&object, None)
        .map_err(|e| format!("Failed to checkout tree: {}", e))?;
    repository.set_head(&format!("refs/heads/{}", branch_name))
      .map_err(|e| format!("Failed to set head: {}", e))?;

    Ok(branch)
}

pub fn stage_changes(repository: &Repository, file_changes: &Vec<FileChange>) -> Result<(), String> {
    let mut index = repository.index()
        .map_err(|e| format!("Failed to get index: {}", e))?;

    for file_change in file_changes {
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

    index.write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    Ok(())
}

pub fn get_file_changes(repository: &Repository, include_unstaged: bool) -> Result<Vec<FileChange>, String> {
    let mut result = Vec::new();
    let repository_workdir = repository.workdir()
        .ok_or("Failed to get workdir from repository".to_string())?;

    let statuses = repository.statuses(None)
        .map_err(|e| format!("Failed to get statuses: {}", e))?;
    for entry in statuses.iter() {
        let status = entry.status();
        let relative_path = PathBuf::from(String::from_utf8_lossy(entry.path_bytes()).to_string());
        let absolute_path_str = repository_workdir.join(&relative_path).to_string_lossy().to_string();
        let absolute_path = to_pathbuf_normalize(&absolute_path_str);

        if status.contains(Status::INDEX_NEW) || 
           (include_unstaged && status.contains(Status::WT_NEW)) {
            result.push(FileChange {
                status: FileChangeStatus::ADDED,
                relative_path: relative_path.clone(),
                absolute_path: absolute_path.clone(),
            });
        }
        if status.contains(Status::INDEX_MODIFIED) || 
           (include_unstaged && status.contains(Status::WT_MODIFIED)) {
            result.push(FileChange {
                status: FileChangeStatus::MODIFIED,
                relative_path: relative_path.clone(),
                absolute_path: absolute_path.clone(),
            });
        }
        if status.contains(Status::INDEX_DELETED) || 
           (include_unstaged && status.contains(Status::WT_DELETED)) {
            result.push(FileChange {
                status: FileChangeStatus::DELETED,
                relative_path: relative_path.clone(),
                absolute_path: absolute_path,
            });
        }
    }

    Ok(result)
}

pub fn get_configured_author_email_and_name(repository: &Repository) -> Result<(String, String), String> {
    let config = repository.config().map_err(|e| format!("Failed to get repository config: {}", e))?;
    let author_email = config.get_string("user.email")
       .map_err(|e| format!("Failed to get author email: {}", e))?;
    let author_name = config.get_string("user.name")
        .map_err(|e| format!("Failed to get author name: {}", e))?;
    Ok((author_email, author_name))
}

pub fn commit(repository: &Repository, branch: &Branch, message: &str, author_name: &str, author_email: &str) -> Result<Oid, String> {

    let mut index = repository.index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    let tree_id = index.write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repository.find_tree(tree_id)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    let signature = Signature::now(author_name, author_email)
        .map_err(|e| format!("Failed to create signature: {}", e))?;

    let branch_ref_name = branch.get().name()
        .ok_or_else(|| "Invalid branch name".to_string())?;

    let parent_commit = if let Some(target) = branch.get().target() {
        repository.find_commit(target)
            .map_err(|e| format!("Failed to find branch commit: {}", e))?
    } else {
        return Err("No parent commits found".to_string());
    };

    repository.commit(
        Some(branch_ref_name), &signature, &signature, message, &tree, &[&parent_commit]
    ).map_err(|e| format!("Failed to create commit: {}", e))
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
        index.add_path(Path::new(&file_change.relative_path))
            .map_err(|e| format!("Failed to add file to index: {}", e))?;
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

        let file_changes = match get_file_changes(&repository, true) {
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

pub fn initialize_repo(workspace_path: &PathBuf, git_dir_path: &PathBuf) -> Result<Repository, String> {
    let repo = git2::Repository::init(&git_dir_path)
        .map_err(|e| format!("Failed to initialize repository: {}", e))?;

    repo.set_workdir(&workspace_path, false)
        .map_err(|e| format!("Failed to set workdir: {}", e))?;

    {
        let tree_id = {
            let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
            index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?
        };
        let tree = repo.find_tree(tree_id).map_err(|e| format!("Failed to find tree: {}", e))?;
        let signature = git2::Signature::now("Refact Agent", "agent@refact.ai")
            .map_err(|e| format!("Failed to create signature: {}", e))?;
        repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[])
            .map_err(|e| format!("Failed to create initial commit: {}", e))?;
    }

    Ok(repo)
}

pub fn clean_and_hard_reset(repo: &Repository, commit_hash: &str) -> Result<(), String> {
    // Clean untracked files and directories
    let statuses = repo.statuses(None)
        .map_err(|e| format!("Failed to get statuses: {}", e))?;
    let workspace_dir = repo.workdir()
        .ok_or("Failed to get workdir from repository".to_string())?;
    for entry in statuses.iter() {
        let status = entry.status();
        if status.contains(git2::Status::WT_NEW) {
            let path = String::from_utf8_lossy(entry.path_bytes()).to_string();
            let full_path = workspace_dir.join(path);
            if full_path.is_dir() {
                std::fs::remove_dir_all(&full_path).map_err(|e| format!("Failed to remove directory: {}", e))?;
            } else {
                std::fs::remove_file(&full_path).map_err(|e| format!("Failed to remove file: {}", e))?;
            }
        }
    }

    // Perform a hard reset to the specified commit
    let oid = Oid::from_str(commit_hash)
        .map_err(|e| format!("Invalid commit hash: {}", e))?;
    let obj = repo.find_object(oid, None)
        .map_err(|e| format!("Failed to find object: {}", e))?;
    repo.reset(&obj, git2::ResetType::Hard, None)
        .map_err(|e| format!("Failed to perform hard reset: {}", e))?;

    Ok(())
}

fn get_workspace_and_commit_hash_from_checkpoint(checkpoint: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = checkpoint.split('/').collect();
    match parts.as_slice() {
        [workspace_hash, commit_hash] => Ok((workspace_hash.to_string(), commit_hash.to_string())),
        [""] => Ok(("".to_string(), "".to_string())),
        _ => return Err("Invalid checkpoint".to_string()),
    }
}

pub async fn create_workspace_checkpoint(gcx: Arc<ARwLock<GlobalContext>>, last_checkpoint: &str, chat_id: &str) -> Result<String, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
        .ok_or_else(|| "No active workspace folder".to_string())?;
    let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());

    let (last_check_workspace_hash, _) = get_workspace_and_commit_hash_from_checkpoint(last_checkpoint)?;
    if !last_check_workspace_hash.is_empty() && last_check_workspace_hash != workspace_folder_hash {
        return Err("Can not create checkpoint for different workspace folder".to_string());
    }

    let shadow_repo_path  = cache_dir.join("shadow_git").join(&workspace_folder_hash);
    let repo = match git2::Repository::open(&shadow_repo_path) {
        Ok(repo) => {
            repo.set_workdir(&workspace_folder, false)
                .map_err(|e| format!("Failed to set workdir: {}", e))?;
            Ok(repo)
        },
        Err(e) => {
            if e.code() == git2::ErrorCode::NotFound {
                initialize_repo(&workspace_folder, &shadow_repo_path)
            } else {
                Err(format!("Failed to open repository: {}", e))
            }
        },
    }?;

    let file_changes = get_file_changes(&repo, true)?;
    stage_changes(&repo, &file_changes)?;

    let branch = create_or_checkout_to_branch(&repo, &format!("refact-{chat_id}"))?;
    let commit_oid = commit(&repo, &branch, &format!("Auto commit for chat {chat_id}"), "Refact Agent", "agent@refact.ai")?;

    Ok(format!("{workspace_folder_hash}/{commit_oid}"))
}

pub async fn restore_workspace_checkpoint(gcx: Arc<ARwLock<GlobalContext>>, checkpoint: &str) -> Result<(), String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let workspace_folder = get_active_workspace_folder(gcx.clone()).await
       .ok_or_else(|| "No active workspace folder".to_string())?;
    let workspace_folder_hash = official_text_hashing_function(&workspace_folder.to_string_lossy().to_string());

    let (checkpoint_workspace_hash, checkpoint_commit_hash) = 
        get_workspace_and_commit_hash_from_checkpoint(checkpoint)?;

    if !checkpoint_workspace_hash.is_empty() && checkpoint_workspace_hash != workspace_folder_hash {
        return Err("Can not restore checkpoint for different workspace folder".to_string());
    }

    let shadow_repo_path  = cache_dir.join("shadow_git").join(&workspace_folder_hash);

    let repo = git2::Repository::open(&shadow_repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;
    repo.set_workdir(&workspace_folder, false)
        .map_err(|e| format!("Failed to set workdir: {}", e))?;

    clean_and_hard_reset(&repo, &checkpoint_commit_hash)?;
    Ok(())
}