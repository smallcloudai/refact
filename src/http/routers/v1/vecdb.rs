use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::structs::VecdbSearch;

#[derive(Serialize, Deserialize, Clone)]
struct VecDBPost {
    query: String,
    top_n: usize,
}

const NO_VECDB: &str = "Vector db is not running, check if you have --vecdb parameter and a vectorization model is running on server side.";


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
                StatusCode::INTERNAL_SERVER_ERROR, NO_VECDB.to_string()
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
        Some(ref db) => match db.get_status().await {
            Ok(status) => status,
            Err(err) => {
                return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, err));
            }
        },
        None => {
            return Err(ScratchError::new(
                StatusCode::INTERNAL_SERVER_ERROR, NO_VECDB.to_string()
            ));
        }
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(&status).unwrap()))
        .unwrap())
}
