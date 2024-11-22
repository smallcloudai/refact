use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use git2::Repository;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use url::Url;

use crate::custom_error::ScratchError;
use crate::git::{commit, count_file_changes, create_or_checkout_to_branch, stage_all_changes};
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitStageAndCommitPost {
    chat_id: String,
    repository_path: Url,
}

pub async fn handle_v1_git_stage_and_commit(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<GitStageAndCommitPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let repo_path = crate::files_correction::canonical_path(
        &post.repository_path.to_file_path().unwrap_or_default().to_string_lossy().to_string());
    let repository = Repository::open(&repo_path)
      .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Could not open repository: {}", e)))?;

    let branch_name = format!("refact-{}", post.chat_id);
    let branch = create_or_checkout_to_branch(&repository, &branch_name)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    stage_all_changes(&repository)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let (new_files, modified_files, deleted_files) = count_file_changes(&repository)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let commit_oid = if new_files + modified_files + deleted_files != 0 {
        Some(commit(
            &repository, 
            &branch,
            &format!("Refact agent commit in chat {} at {}", post.chat_id, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
            "Refact Agent",
            "agent@refact.ai",
        ).map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?)
    } else {
        None
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::json!({
            "commit_oid": commit_oid.map(|x| x.to_string()),
            "new_files": new_files,
            "modified_files": modified_files,
            "deleted_files": deleted_files,
        }).to_string()))
        .unwrap())
}