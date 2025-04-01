use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;
use crate::providers::{read_providers_d, get_provider_templates};

pub async fn handle_v1_providers(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Response<Body> {
    let config_dir = {
        let gcx_locked = gcx.read().await;
        gcx_locked.config_dir.clone()
    };

    let (providers, read_errors) = read_providers_d(Vec::new(), &config_dir).await;
    
    let result = providers.into_iter().map(|p| { json!({
        "name": p.name,
        "enabled": p.enabled
    })}).collect::<Vec<_>>();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({
            "providers": result,
            "error_log": read_errors
        })).unwrap()))
        .unwrap()
}

pub async fn handle_v1_provider_templates() -> Response<Body> {
    let provider_templates = get_provider_templates();
    
    let result = provider_templates.keys().map(|name| { json!({
        "name": name
    })}).collect::<Vec<_>>();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({
            "provider_templates": result
        })).unwrap()))
        .unwrap()
}
