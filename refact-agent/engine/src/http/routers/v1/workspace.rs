use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveGroupIdPost {
    pub group_id: usize,
}


pub async fn handle_v1_set_active_group_id(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SetActiveGroupIdPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    gcx.write().await.active_group_id = Some(post.group_id);
    
    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true })).unwrap()
    )).unwrap())
}
