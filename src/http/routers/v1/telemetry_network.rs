use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde_json::json;

use crate::telemetry::telemetry_structs;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

pub async fn handle_v1_telemetry_network(
    Extension(global_context): Extension<SharedGlobalContext<'_>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<telemetry_structs::TelemetryNetwork>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    global_context.write().await.telemetry.write().unwrap().tele_net.push(post);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({"success": 1}).to_string()))
        .unwrap())
}
