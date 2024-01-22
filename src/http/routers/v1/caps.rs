use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

const MAX_CAPS_AGE: u64 = 10;


pub async fn handle_v1_caps(
    Extension(global_context): Extension<SharedGlobalContext<'_>>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let caps_result = crate::global_context::try_load_caps_quickly_if_not_present(
        global_context.clone(),
        MAX_CAPS_AGE,
    ).await;
    let caps_arc = match caps_result {
        Ok(x) => x,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::SERVICE_UNAVAILABLE, format!("{}", e)));
        }
    };
    let caps_locked = caps_arc.read().unwrap();
    let body = serde_json::to_string_pretty(&*caps_locked).unwrap();
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Ok(response)
}
