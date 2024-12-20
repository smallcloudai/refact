use axum::response::Result;
use axum::Extension;
use serde_json::json;
use hyper::{Body, Response, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tracing::error;

use crate::global_context::GlobalContext;
use crate::custom_error::ScratchError;
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
	let tconfig = match load_customization(global_context.clone(), false).await {
		Ok(config) => config,
		Err(err) => {
			error!("load_customization: {}", err);
			return Ok(Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from(serde_json::to_string_pretty(&json!({ "detail": err.to_string() })).unwrap()))
				.unwrap());
		}
	};
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(&tconfig).unwrap()))
        .unwrap())
}
