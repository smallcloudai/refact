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


pub async fn handle_v1_trajectory_compress(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CompressTrajectoryPost>(&body_bytes).map_err(|e| {
        ScratchError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("JSON problem: {}", e),
        )
    })?;

    let (goal, trajectory) = compress_trajectory(global_context.clone(), &post.messages)
        .await.map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    let response = serde_json::json!({
        "goal": goal,
        "trajectory": trajectory,
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
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
    let (memdb, vectorizer_service) = {
        let gcx_locked = global_context.read().await;
        let memdb = gcx_locked.memdb.clone()
            .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string()))?;
        let vectorizer_service = gcx_locked.vectorizer_service.clone()
            .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "vectorizer_service not initialized".to_string()))?;
        (memdb, vectorizer_service)
    };
    let mem_type = "trajectory";
    let (goal, trajectory) = compress_trajectory(global_context.clone(), &post.messages)
        .await.map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let memid = crate::memdb::db_memories::memories_add(
        memdb,
        vectorizer_service,
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
