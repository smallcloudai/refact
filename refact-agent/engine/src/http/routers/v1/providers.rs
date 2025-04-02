use axum::extract::Query;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::caps::DefaultModels;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::providers::{get_provider_templates, read_providers_d, CapsProvider};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderDTO {
    endpoint_style: String,
    chat_endpoint: String,
    completion_endpoint: String,
    embedding_endpoint: String,
    api_key: String,
    
    #[serde(flatten)]
    defaults: DefaultModels,
    
    enabled: bool,
}

impl ProviderDTO {
    pub fn from_caps_provider(provider: CapsProvider) -> Self {
        ProviderDTO {
            endpoint_style: provider.endpoint_style,
            chat_endpoint: provider.chat_endpoint,
            completion_endpoint: provider.completion_endpoint,
            embedding_endpoint: provider.embedding_endpoint,
            api_key: provider.api_key,
            defaults: provider.defaults,
            enabled: provider.enabled,
        }
    }
}

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

#[derive(Deserialize)]
pub struct GetProviderQueryParams {
    #[serde(rename = "provider-name")]
    provider_name: String,
}

pub async fn handle_v1_get_provider(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<GetProviderQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let config_dir = gcx.read().await.config_dir.clone();
    let provider_path = config_dir.join("providers.d").join(format!("{}.yaml", params.provider_name));

    let mut provider = get_provider_templates().get(&params.provider_name).cloned()
        .ok_or(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "Provider template not found".to_string()))?;

    let file_content = tokio::fs::read_to_string(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error reading provider file: {}", e)))?;

    let file_value = serde_yaml::from_str::<serde_yaml::Value>(&file_content)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error parsing provider file: {}", e)))?;

    provider.apply_override(file_value)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error applying provider override: {}", e)))?;
    
    let provider_dto = ProviderDTO::from_caps_provider(provider);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&provider_dto).unwrap()))
        .unwrap())
}

