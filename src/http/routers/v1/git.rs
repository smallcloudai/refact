use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use git2::Repository;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use url::Url;

use crate::custom_error::ScratchError;
use crate::git::{get_configured_author_email_and_name, restore_workspace_checkpoint, stage_changes, CommitInfo};
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Debug)]
pub struct GitCommitPost {
    pub commits: Vec<CommitInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitError {
    pub error_message: String,
    pub project_name: String,
    pub project_path: Url,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRollbackPost {
    pub revision: String,
}

pub async fn handle_v1_git_commit(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<GitCommitPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let mut error_log = Vec::new();
    let mut commits_applied = Vec::new();

    for commit in post.commits {
        let repo_path = crate::files_correction::to_pathbuf_normalize(
            &commit.project_path.to_file_path().unwrap_or_default().display().to_string());

        let project_name = commit.project_path.to_file_path().ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "".to_string());

        let git_error = |msg: String| -> GitError {
            GitError {
                error_message: msg,
                project_name: project_name.clone(),
                project_path: commit.project_path.clone(),
            }
        };

        let repository = match Repository::open(&repo_path) {
            Ok(repo) => repo,
            Err(e) => { error_log.push(git_error(format!("Failed to open repo: {}", e))); continue; }
        };

        if let Err(stage_err) = stage_changes(&repository, &commit.file_changes) {
            error_log.push(git_error(stage_err));
            continue;
        }
        
        let (author_email, author_name) = match get_configured_author_email_and_name(&repository) {
            Ok(email_and_name) => email_and_name,
            Err(err) => { 
                error_log.push(git_error(err));
                continue; 
            }
        };
        
        let branch = match repository.head().map(|reference| git2::Branch::wrap(reference)) {
            Ok(branch) => branch,
            Err(e) => { error_log.push(git_error(format!("Failed to get current branch: {}", e))); continue; }
        };
        
        let commit_oid = match crate::git::commit(&repository, &branch, &commit.commit_message, &author_name, &author_email) {
            Ok(oid) => oid,
            Err(e) => { error_log.push(git_error(e)); continue; }
        };

        commits_applied.push(serde_json::json!({
            "project_name": project_name,
            "project_path": commit.project_path.to_string(),
            "commit_oid": commit_oid.to_string(),
        }));
    }
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&serde_json::json!({
            "commits_applied": commits_applied,
            "error_log": error_log,
        })).unwrap()))
        .unwrap())
}

pub async fn handle_v1_rollback_changes(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<GitRollbackPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    restore_workspace_checkpoint(gcx.clone(), &post.revision).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::json!({"success": true}).to_string()))
        .unwrap())
}