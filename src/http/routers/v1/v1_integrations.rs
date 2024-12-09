use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use axum::extract::Path;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


pub async fn handle_v1_integrations(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let integrations = crate::integrations::setting_up_integrations::integrations_all(gcx.clone()).await;
    let payload = serde_json::to_string_pretty(&integrations).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize payload: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}

#[derive(Deserialize)]
struct IntegrationGetPost {
    pub integr_config_path: String,
}

pub async fn handle_v1_integration_get(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<IntegrationGetPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let the_get = crate::integrations::setting_up_integrations::integration_config_get(
        post.integr_config_path,
    ).await.map_err(|e|{
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load integrations: {}", e))
    })?;

    let payload = serde_json::to_string_pretty(&the_get).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize payload: {}", e))
    })?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap())
}

#[derive(Deserialize)]
struct IntegrationSavePost {
    pub integr_config_path: String,
    pub integr_values: serde_json::Value,
}

pub async fn handle_v1_integration_save(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<IntegrationSavePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    crate::integrations::setting_up_integrations::integration_config_save(&post.integr_config_path, &post.integr_values).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    Ok(Response::builder()
       .status(StatusCode::OK)
       .header("Content-Type", "application/json")
       .body(Body::from(format!("")))
       .unwrap())
}

mod generated {
    include!(concat!(env!("OUT_DIR"), "/available_icons.rs"));
}

pub async fn handle_v1_integration_icon(
    Path(icon_name): Path<String>,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let icons = generated::get_available_icons();
    let sanitized_icon_name = icon_name
        .split('/').last()
        .map(|x| x.replace("_TEMPLATE", "")).ok_or(
        ScratchError::new(StatusCode::BAD_REQUEST, "invalid file name".to_string())
    )?;
    if let Some(icon_bytes) = icons.get(sanitized_icon_name.as_str()) {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .header("Content-Disposition", "inline")
            .body(Body::from(*icon_bytes))
            .unwrap());
    }
    Err(ScratchError::new(StatusCode::NOT_FOUND, "icon not found".to_string()))
}
