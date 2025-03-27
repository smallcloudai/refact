use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


pub async fn handle_v1_ping(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let ping_message: String = gcx.read().await.cmdline.ping_message.clone();
    let response = Response::builder()
       .header("Content-Type", "application/json")
       .body(Body::from(ping_message + "\n"))
      .unwrap();
    Ok(response)
}

pub async fn handle_v1_caps(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let caps_result = crate::global_context::try_load_caps_quickly_if_not_present(
        global_context.clone(),
        0,
    ).await;
    let caps_arc = match caps_result {
        Ok(x) => x,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::SERVICE_UNAVAILABLE, format!("{}", e)));
        }
    };
    let body = serde_json::to_string_pretty(&*caps_arc).unwrap();
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Ok(response)
}
