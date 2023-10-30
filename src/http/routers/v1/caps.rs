use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde_json::json;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

pub async fn handle_v1_caps(
    Extension(global_context): Extension<SharedGlobalContext>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let caps_result = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone()).await;
    let caps = match caps_result {
        Ok(x) => x,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::SERVICE_UNAVAILABLE, format!("{}", e)));
        }
    };
    let caps_locked = caps.read().unwrap();
    let body = json!(*caps_locked).to_string();
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Ok(response)
}
