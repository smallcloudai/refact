use std::collections::HashSet;
use std::sync::Arc;
use axum::Extension;
use axum::http::Response;
use hyper::{Body, StatusCode};
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_file::file_repair_candidates;
use crate::custom_error::ScratchError;
use crate::files_correction::correct_to_nearest_dir_path;
use crate::global_context::GlobalContext;

#[derive(Deserialize)]
struct ResolveShortenedPathPost {
    path: String
}

pub async fn handle_v1_resolve_shortened_path(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<ResolveShortenedPathPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let candidates_file = file_repair_candidates(gcx.clone(), &post.path, 10, false).await;
    let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &post.path, false, 10).await;
    let resp = candidates_file.into_iter().chain(candidates_dir.into_iter()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&resp).unwrap()))
        .unwrap())
}
