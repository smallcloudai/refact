use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response};
use serde_json::json;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

pub async fn handle_v1_graceful_shutdown(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let gcx_locked = global_context.read().await;
    gcx_locked.ask_shutdown_sender.lock().unwrap().send(format!("going-down")).unwrap();
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"success": true}).to_string()))
        .unwrap())
}
