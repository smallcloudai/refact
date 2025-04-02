use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::caps::get_custom_embedding_api_key;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::vecdb::vdb_structs::VecdbSearch;


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
    let (vecdb, vectorizer_service) = {
        let gcx_locked = gcx.read().await;
        let vecdb = gcx_locked.vecdb.clone()
            .ok_or_else(|| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, NO_VECDB.to_string()))?;
        let vectorizer_service = gcx_locked.vectorizer_service.clone();
        (vecdb, vectorizer_service)
    };
    let search_res = {
        let vecdb_locked = vecdb.lock().await;
        vecdb_locked.vecdb_search(post.query.to_string(), post.top_n, None, &api_key, vectorizer_service.clone()).await
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
    Extension(gcx): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vectorizer_service_mb = gcx.read().await.vectorizer_service.clone();
    let status_str = if let Some(vectorizer_service) = vectorizer_service_mb {
        match crate::vecdb::vdb_highlev::get_status(vectorizer_service).await {
            Ok(Some(status)) => serde_json::to_string_pretty(&status).unwrap(),
            Ok(None) => "{\"success\": 0, \"detail\": \"turned_off\"}".to_string(),
            Err(err) => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, err))
        }
    } else {
        "{\"success\": 0, \"detail\": \"turned_off\"}".to_string()
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(status_str))
        .unwrap())
}
