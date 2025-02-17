use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use axum::http::{Response, StatusCode};
use axum::Extension;
use hyper::Body;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::agentic::compress_trajectory::compress_trajectory;
use crate::call_validation::ChatMessage;

#[derive(Deserialize)]
struct CommitMessageFromDiffPost {
    diff: String,
    #[serde(default)]
    text: Option<String>, // a prompt for the commit message
}

pub async fn handle_v1_commit_message_from_diff(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CommitMessageFromDiffPost>(&body_bytes).map_err(|e| {
        ScratchError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("JSON problem: {}", e),
        )
    })?;

    let commit_message = generate_commit_message_by_diff(global_context.clone(), &post.diff, &post.text)
        .await
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(commit_message))
        .unwrap())
}

#[derive(Deserialize)]
struct CompressTrajectoryPost {
    project: String,
    messages: Vec<ChatMessage>,
}

pub async fn handle_v1_trajectory_save(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CompressTrajectoryPost>(&body_bytes).map_err(|e| {
        ScratchError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("JSON problem: {}", e),
        )
    })?;

    let mem_type = "trajectory";
    let (goal, trajectory) = compress_trajectory(global_context.clone(), &post.messages)
        .await.map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    let vec_db = global_context.read().await.vec_db.clone();
    let memid = crate::vecdb::vdb_highlev::memories_add(
        vec_db,
        &mem_type,
        &goal.as_str(),
        &post.project.as_str(),
        &trajectory.as_str(),
        "local-compressed-traj",
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response = serde_json::json!({
        "memid": memid,
        "trajectory": trajectory,
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
