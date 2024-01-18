use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::structs::VecdbSearch;
use tracing::info;

#[derive(Serialize, Deserialize, Clone)]
struct VecDBPost {
    query: String,
    top_n: usize,
}

pub async fn handle_v1_vecdb_search(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<VecDBPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let cx_locked = global_context.read().await;
    let search_res = match *cx_locked.vec_db.lock().await {
        Some(ref db) => db.search(post.query.to_string(), post.top_n).await,
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Vector db is not available".to_string()
            ));
        }
    };

    match search_res {
        Ok(search_res) => {
            let json_string = serde_json::to_string_pretty(&search_res).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
        Err(e) => {
            Err(ScratchError::new(StatusCode::BAD_REQUEST, e))
        }
    }
}

pub async fn handle_v1_vecdb_status(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let cx_locked = global_context.read().await;
    let status = match *cx_locked.vec_db.lock().await {
        Some(ref db) => db.get_status().await,
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, "Vector db is not available".to_string()
            ));
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!(status).to_string()))
        .unwrap())
}

pub async fn handle_v1_vecdb_caps(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let caps = {
        let cx_locked = global_context.read().await;
        let db = cx_locked.vec_db.lock().await;
        if let Some(ref db) = *db {
            db.caps().await
        } else {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Vector db is not available".to_string()
            ));
        }
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!(caps).to_string()))
        .unwrap())
}