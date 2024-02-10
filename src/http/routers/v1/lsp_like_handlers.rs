use std::path::PathBuf;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::receive_workspace_changes;
use crate::vecdb::file_filter::retrieve_files_by_proj_folders;

#[derive(Serialize, Deserialize, Clone)]
struct PostInit {
    pub project_roots: Vec<Url>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PostDocument {
    pub uri: Url,
    pub text: String,
}


pub async fn handle_v1_lsp_initialize(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PostInit>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let files = retrieve_files_by_proj_folders(
        post.project_roots.iter().map(|x| PathBuf::from(x.path())).collect()
    ).await;
    let binding = global_context.read().await;
    match *binding.vec_db.lock().await {
        Some(ref mut db) => db.add_or_update_files(&files, true).await,
        None => {}
    };
    match *binding.ast_module.lock().await {
        Some(ref mut ast) => ast.add_or_update_files(&files, true).await,
        None => {}
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1}).to_string()))
        .unwrap())
}

pub async fn handle_v1_lsp_did_change(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PostDocument>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let path = PathBuf::from(post.uri.path());
    let binding = global_context.read().await;
    match *binding.vec_db.lock().await {
        Some(ref mut db) => db.add_or_update_file(path.clone(), false).await,
        None => {}
    };
    match *binding.ast_module.lock().await {
        Some(ref mut ast) => ast.add_or_update_file(path, false).await,
        None => {}
    };
    receive_workspace_changes::on_did_change(
        global_context.clone(),
        &post.uri.to_string(),
        &post.text,
    ).await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1}).to_string()))
        .unwrap())
}
