use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{warn, error};

use crate::caps::get_custom_embedding_api_key;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::vdb_structs::{VecdbSearch, SearchResult};


#[derive(Serialize, Deserialize, Clone)]
struct VecDBPost {
    query: String,
    top_n: usize,
}

const NO_VECDB: &str = "Vector db is not running, check if you have --vecdb parameter and a vectorization model is running on server side.";


pub async fn handle_v1_vecdb_search(
    Extension(gcx): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<VecDBPost>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let api_key = get_custom_embedding_api_key(gcx.clone()).await?;
    let cx_locked = gcx.read().await;

    // Create an empty result in case we need to degrade gracefully
    let empty_result = SearchResult {
        query_text: post.query.to_string(),
        results: vec![],
    };

    // Instead of failing when vec_db is None, we degrade gracefully
    let search_res = match *cx_locked.vec_db.lock().await {
        Some(ref db) => db.vecdb_search(post.query.to_string(), post.top_n, None, &api_key).await,
        None => {
            // Log a warning and return an empty search result
            warn!("Vector database not active, returning empty search result");
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(serde_json::to_string_pretty(&empty_result).unwrap()))
                .unwrap());
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
            // Instead of returning an error that makes the vecdb search completely unusable,
            // we report the error and return an empty result.
            error!("vecdb search error: {}", e);
            
            let json_string = serde_json::to_string_pretty(&empty_result).map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("JSON serialization problem: {}", e))
            })?;
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(json_string))
                .unwrap())
        }
    }
}


pub async fn handle_v1_vecdb_status(
    Extension(gcx): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vec_db = gcx.read().await.vec_db.clone();
    let status_str = match crate::vecdb::vdb_highlev::get_status(vec_db).await {
        Ok(Some(status)) => serde_json::to_string_pretty(&status).unwrap(),
        Ok(None) => "{\"success\": 0, \"detail\": \"turned_off\"}".to_string(),
        Err(err) => {
            // Log the error but return a safe status instead of failing
            error!("Failed to get vecdb status: {}", err);
            "{\"success\": 0, \"detail\": \"error\", \"message\": \"Failed to get status\"}".to_string()
        }
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(status_str))
        .unwrap())
}

