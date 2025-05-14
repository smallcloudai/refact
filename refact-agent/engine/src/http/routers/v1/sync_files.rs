use std::path::PathBuf;
use std::sync::Arc;
use tokio_tar::ArchiveBuilder;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncFilesExtractTarPost {
    pub tar_path: String,
    pub extract_to: String,
}

pub async fn handle_v1_sync_files_extract_tar(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SyncFilesExtractTarPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let (tar_path, extract_to) = (PathBuf::from(&post.tar_path), PathBuf::from(&post.extract_to));

    let tar_file = tokio::fs::File::open(&tar_path).await
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Can't open tar file: {}", e)))?;

    ArchiveBuilder::new(tar_file)
        .set_preserve_permissions(true)
        .build()
        .unpack(&extract_to).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Can't unpack tar file: {}", e)))?;   

    tokio::fs::remove_file(&tar_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Can't remove tar file: {}", e)))?;

    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true })).unwrap()
    )).unwrap())
}