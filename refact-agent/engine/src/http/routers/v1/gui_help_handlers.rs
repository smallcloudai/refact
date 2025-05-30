use std::collections::HashSet;
use std::sync::Arc;
use axum::Extension;
use axum::http::Response;
use hyper::{Body, StatusCode};
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::custom_error::ScratchError;
use crate::files_correction::{correct_to_nearest_dir_path, preprocess_path_for_normalization};
use crate::global_context::GlobalContext;

#[derive(Deserialize)]
struct ResolveShortenedPathPost {
    path: String
}

pub async fn handle_v1_fullpath(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<ResolveShortenedPathPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let path = preprocess_path_for_normalization(post.path);
    let candidates_file = file_repair_candidates(gcx.clone(), &path, 10, false).await;
    let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &path, false, 10).await;
    let candidates = candidates_file.into_iter().chain(candidates_dir.clone().into_iter()).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

    match return_one_candidate_or_a_good_error(gcx.clone(), &path, &candidates, &vec![], false).await {
        Ok(candidate) => {
            let is_directory = candidates_dir.contains(&candidate);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string_pretty(&serde_json::json!({
                    "fullpath": candidate,
                    "is_directory": is_directory
                })).unwrap()))
                .unwrap())
        },
        Err(err) => Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string_pretty(&serde_json::json!({ "detail": err })).unwrap()))
            .unwrap()),
    }
}
