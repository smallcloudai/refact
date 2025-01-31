use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::RwLock as ARwLock;
use std::path::PathBuf;
use url::Url;
use tracing::{error, info, warn};

use crate::global_context::GlobalContext;
use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::git::CommitInfo;
use crate::git::operations::{get_diff_statuses_worktree_to_head, git_diff_as_string};

pub async fn get_commit_information_from_current_changes(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<CommitInfo>
{
    let mut commits = Vec::new();

    let workspace_vcs_roots: Arc<StdMutex<Vec<PathBuf>>> = {
        let cx_locked = gcx.write().await;
        cx_locked.documents_state.workspace_vcs_roots.clone()
    };

    let vcs_roots_locked = workspace_vcs_roots.lock().unwrap();
    info!("get_commit_information_from_current_changes() vcs_roots={:?}", vcs_roots_locked);
    for project_path in vcs_roots_locked.iter() {
        let repository = match git2::Repository::open(project_path) {
            Ok(repo) => repo,
            Err(e) => { warn!("{}", e); continue; }
        };

        let file_changes = match get_diff_statuses_worktree_to_head(&repository, true) {
            Ok(changes) if changes.is_empty() => { continue; }
            Ok(changes) => changes,
            Err(e) => { warn!("{}", e); continue; }
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