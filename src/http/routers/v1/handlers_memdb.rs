use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use serde_json::json;
use indexmap::IndexMap;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::Deserialize;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


#[derive(Deserialize)]
struct MemAddRequest {
    mem_type: String,
    goal: String,
    project: String,
    payload: String,   // TODO: upgrade to serde_json::Value
}

#[derive(Deserialize)]
struct MemEraseRequest {
    memid: String,
}

#[derive(Deserialize)]
struct MemUpdateUsedRequest {
    memid: String,
    correct: i32,
    relevant: i32,
}

#[derive(Deserialize)]
struct MemQuery {
    goal: String,
    #[allow(unused)]
    project: String,
    top_n: usize,
}

pub async fn handle_mem_add(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemAddRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let vec_db = gcx.read().await.vec_db.clone();
    let memid = crate::vecdb::vdb_highlev::memories_add(
        vec_db,
        &post.mem_type,
        &post.goal,
        &post.project,
        &post.payload
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"memid": memid})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_erase(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemEraseRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let vec_db = gcx.read().await.vec_db.clone();
    let erased_cnt = crate::vecdb::vdb_highlev::memories_erase(vec_db, &post.memid).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    assert!(erased_cnt <= 1);

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": erased_cnt>0})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_update_used(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemUpdateUsedRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let vec_db = gcx.read().await.vec_db.clone();
    let updated_cnt = crate::vecdb::vdb_highlev::memories_update(
        vec_db,
        &post.memid,
        post.correct,
        post.relevant
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    assert!(updated_cnt <= 1);

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": updated_cnt>0})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_block_until_vectorized(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vec_db = gcx.read().await.vec_db.clone();
    crate::vecdb::vdb_highlev::memories_block_until_vectorized(vec_db)
        .await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": true})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_query(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemQuery = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let vec_db = gcx.read().await.vec_db.clone();
    let memories = crate::vecdb::vdb_highlev::memories_search(
        vec_db,
        &post.goal,
        post.top_n
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
    })?;

    let response_body = serde_json::to_string_pretty(&memories).unwrap();

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(response_body))
        .unwrap();
    Ok(response)
}

pub async fn handle_mem_list(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vec_db = gcx.read().await.vec_db.clone();

    let memories = crate::vecdb::vdb_highlev::memories_select_all(vec_db).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response_body = serde_json::to_string_pretty(&memories).unwrap();

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(response_body))
        .unwrap();

    Ok(response)
}

#[derive(Deserialize)]
struct OngoingUpdateRequest {
    goal: String,
    ongoing_progress: IndexMap<String, serde_json::Value>,
    ongoing_action_new_sequence: IndexMap<String, serde_json::Value>,
    ongoing_output: IndexMap<String, IndexMap<String, serde_json::Value>>,
}

pub async fn handle_ongoing_update_or_create(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: OngoingUpdateRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let vec_db = gcx.read().await.vec_db.clone();

    crate::vecdb::vdb_highlev::ongoing_update_or_create(
        vec_db,
        post.goal,
        post.ongoing_progress,
        post.ongoing_action_new_sequence,
        post.ongoing_output,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": true})).unwrap()))
        .unwrap();
    Ok(response)
}

pub async fn handle_ongoing_dump(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vec_db = gcx.read().await.vec_db.clone();
    let output = crate::vecdb::vdb_highlev::ongoing_dump(vec_db).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;
    let response = Response::builder()
        .header("Content-Type", "text/plain")
        .body(Body::from(output))
        .unwrap();
    Ok(response)
}
