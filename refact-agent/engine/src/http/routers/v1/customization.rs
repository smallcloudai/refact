use axum::response::Result;
use axum::Extension;
use hyper::{Body, Response, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;
use crate::custom_error::{ScratchError, YamlError};
use crate::yaml_configs::customization_loader::load_customization;


pub async fn handle_v1_config_path(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let config_dir = global_context.read().await.config_dir.clone();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(config_dir.to_str().unwrap().to_string()))
        .unwrap())
}

pub async fn handle_v1_customization(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mut error_log: Vec<YamlError> = Vec::new();
    let tconfig = load_customization(global_context.clone(), false, &mut error_log).await;

    let mut response_body = serde_json::to_value(tconfig).unwrap();
    response_body["error_log"] = serde_json::to_value(error_log).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(&response_body).unwrap()))
        .unwrap())
}
