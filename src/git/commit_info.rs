use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use url::Url;
use tracing::{error, info, warn};

use crate::global_context::GlobalContext;
use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::git::CommitInfo;
use crate::git::operations::{get_diff_statuses_workdir_to_head, git_diff_as_string};

pub async fn get_commit_information_from_current_changes(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<CommitInfo>
{
    let mut commits = Vec::new();

    let workspace_vcs_roots_arc = gcx.read().await.documents_state.workspace_vcs_roots.clone();
    let workspace_vcs_roots = workspace_vcs_roots_arc.lock().unwrap().clone();

    info!("get_commit_information_from_current_changes() vcs_roots={:?}", workspace_vcs_roots);
    for project_path in workspace_vcs_roots {
        let repository = match git2::Repository::open(&project_path) {
            Ok(repo) => repo,
            Err(e) => { warn!("{}", e); continue; }
        };

        let file_changes = match get_diff_statuses_workdir_to_head(&repository) {
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