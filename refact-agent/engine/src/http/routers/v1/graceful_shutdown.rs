use axum::Extension;
use hyper::{Body, Response};
use serde_json::json;

use crate::global_context::SharedGlobalContext;

pub async fn handle_v1_graceful_shutdown(
    Extension(global_context): Extension<SharedGlobalContext>,
) -> Response<Body> {
    let gcx_locked = global_context.read().await;
    gcx_locked
        .ask_shutdown_sender
        .lock()
        .unwrap()
        .send("going-down".to_string())
        .unwrap();
    Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"success": true}).to_string()))
        .unwrap()
}
