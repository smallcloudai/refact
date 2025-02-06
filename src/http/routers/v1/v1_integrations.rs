use std::path::PathBuf;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use regex::Regex;
use axum::extract::Path;
use axum::extract::Query;
use rust_embed::RustEmbed;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::json_schema::INTEGRATION_JSON_SCHEMA;
use crate::integrations::setting_up_integrations::split_path_into_project_and_integration;


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

pub async fn handle_v1_integrations_filtered(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(integr_name): Path<String>,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let integrations_result: crate::integrations::setting_up_integrations::IntegrationResult = crate::integrations::setting_up_integrations::integrations_all(gcx.clone()).await;
    let mut filtered_integrations = Vec::new();

    for integration in &integrations_result.integrations {
        let pattern = integration.integr_name.replace("_TEMPLATE", "_.*");
        match Regex::new(&pattern) {
            Ok(re) => {
                if re.is_match(&integr_name) {
                    let mut integration_copy = integration.clone();
                    integration_copy.integr_name = integr_name.clone();
                    if let Some(pos) = integration.integr_config_path.rfind(&integration.integr_name) {
                        let (start, end) = integration.integr_config_path.split_at(pos);
                        integration_copy.integr_config_path = format!("{}{}{}", start, integr_name, &end[integration.integr_name.len()..]);
                    }
                    if integration.integr_name.find("_TEMPLATE").is_some() {
                        let config_path_exists = integrations_result.integrations.iter().any(|existing_integration| {
                            existing_integration.integr_config_path == integration_copy.integr_config_path
                        });
                        if config_path_exists {
                            continue;
                        }
                    }
                    filtered_integrations.push(integration_copy);
                }
            }
            Err(e) => {
                return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("Invalid regex pattern: {}", e)));
            }
        }
    }

    let payload = serde_json::to_string_pretty(&crate::integrations::setting_up_integrations::IntegrationResult {
        integrations: filtered_integrations,
        error_log: integrations_result.error_log,
    }).map_err(|e| {
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

#[derive(RustEmbed)]
#[folder = "assets/integrations/"]
struct IntegrationAsset;

pub async fn handle_v1_integration_icon(
    Path(icon_name): Path<String>,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let sanitized_icon_name = icon_name
        .split('/').last()
        .map(|x| x.replace("_TEMPLATE", "")).ok_or(
        ScratchError::new(StatusCode::BAD_REQUEST, "invalid file name".to_string())
    )?;
    if let Some(icon_bytes) = IntegrationAsset::get(&sanitized_icon_name).map(|file| file.data) {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .header("Content-Disposition", "inline")
            .body(Body::from(icon_bytes))
            .unwrap());
    }
    Err(ScratchError::new(StatusCode::NOT_FOUND, format!("icon {} not found", sanitized_icon_name)))
}

// Define a structure to match query parameters
#[derive(Deserialize)]
pub struct HTTPIntegrationDeleteQueryParams {
    integration_path: PathBuf
}

pub async fn handle_v1_integration_delete(
    Query(params): Query<HTTPIntegrationDeleteQueryParams>,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let integration_path = params.integration_path;
    log::info!("Deleting integration path: {:?}", integration_path);

    split_path_into_project_and_integration(&integration_path).map_err(
        |_| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "integration_path is invalid".to_string())
    )?;

    if !integration_path.exists() {
        return Err(ScratchError::new(StatusCode::NOT_FOUND, "integration_path not found".to_string()));
    }

    std::fs::remove_file(&integration_path).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("failed to delete integration config: {}", e))
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap())
}

pub async fn handle_v1_integration_json_schema() -> axum::response::Result<Response<Body>, ScratchError> {
    let schema_string = serde_json::to_string_pretty(&*INTEGRATION_JSON_SCHEMA).map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize JSON schema: {}", e))
    })?;

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(schema_string))
        .unwrap())
}
