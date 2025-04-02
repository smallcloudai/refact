use axum::extract::Query;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::caps::{DefaultModels, HasBaseModelRecord};
use crate::custom_error::{MapErrToString, ScratchError};
use crate::global_context::GlobalContext;
use crate::providers::{add_name_and_id_to_model_records, add_running_models, apply_models_dict_patch, get_provider_templates, populate_model_records, read_providers_d, CapsProvider};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderDTO {
    name: String,
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
            name: provider.name,
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

#[derive(Serialize)]
pub struct ModelLightResponse {
    name: String,
    enabled: bool,
    deletable: bool,
}

impl ModelLightResponse {
    pub fn new<T: HasBaseModelRecord>(model: T, deletable: bool) -> Self {
        ModelLightResponse {
            name: model.base().name.clone(),
            enabled: model.base().enabled,
            deletable,
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

    let template_names = get_provider_templates().keys().collect::<Vec<_>>();
    let (providers, read_errors) = read_providers_d(Vec::new(), &config_dir).await;
    
    let result = providers.into_iter().filter(
        |p| template_names.contains(&&p.name)
    ).map(|p| { json!({
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

    let file_value = read_yaml_file_as_value_if_exists(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    provider.apply_override(file_value)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error applying provider override: {}", e)))?;
    
    let provider_dto = ProviderDTO::from_caps_provider(provider);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&provider_dto).unwrap()))
        .unwrap())
}

pub async fn handle_v1_post_provider(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let provider_dto = serde_json::from_slice::<ProviderDTO>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error parsing provider: {}", e)))?;

    let config_dir = gcx.read().await.config_dir.clone();
    let provider_path = config_dir.join("providers.d").join(format!("{}.yaml", provider_dto.name));

    let provider_template = get_provider_templates().get(&provider_dto.name).cloned()
        .ok_or(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "Provider template not found".to_string()))?;

    let mut file_value = read_yaml_file_as_value_if_exists(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    update_yaml_field_if_needed(&mut file_value, "endpoint_style", 
        provider_dto.endpoint_style, provider_template.endpoint_style);
    update_yaml_field_if_needed(&mut file_value, "api_key", 
        provider_dto.api_key, provider_template.api_key);
    update_yaml_field_if_needed(&mut file_value, "chat_endpoint",
        provider_dto.chat_endpoint, provider_template.chat_endpoint);
    update_yaml_field_if_needed(&mut file_value, "completion_endpoint",
        provider_dto.completion_endpoint, provider_template.completion_endpoint);
    update_yaml_field_if_needed(&mut file_value, "embedding_endpoint",
        provider_dto.embedding_endpoint, provider_template.embedding_endpoint);
    update_yaml_field_if_needed(&mut file_value, "chat_default_model",
        provider_dto.defaults.chat_default_model, provider_template.defaults.chat_default_model);
    update_yaml_field_if_needed(&mut file_value, "chat_light_model",
        provider_dto.defaults.chat_light_model, provider_template.defaults.chat_light_model);
    update_yaml_field_if_needed(&mut file_value, "chat_thinking_model",
        provider_dto.defaults.chat_thinking_model, provider_template.defaults.chat_thinking_model);
    update_yaml_field_if_needed(&mut file_value, "completion_default_model",
        provider_dto.defaults.completion_default_model, provider_template.defaults.completion_default_model);
    file_value["enabled"] = serde_yaml::Value::Bool(provider_dto.enabled);

    let file_content = serde_yaml::to_string(&file_value)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error parsing provider file: {}", e)))?;
    tokio::fs::write(&provider_path, file_content.as_bytes()).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error writing provider file: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "success": true }).to_string()))
        .unwrap())
}

fn update_yaml_field_if_needed(
    file_value: &mut serde_yaml::Value,
    field_name: &str,
    dto_value: String,
    template_value: String,
) {
    if file_value.get(field_name).is_some() || dto_value != template_value {
        file_value[field_name] = serde_yaml::Value::String(dto_value);
    }
}

async fn read_yaml_file_as_value_if_exists(path: &Path) -> Result<serde_yaml::Value, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            serde_yaml::from_str::<serde_yaml::Value>(&content)
                .map_err_with_prefix("Error parsing file:")
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
        },
        Err(e) => {
            Err(format!("Error reading file: {e}"))
        }
    }
}

pub async fn handle_v1_models(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<GetProviderQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let config_dir = gcx.read().await.config_dir.clone();
    let provider_path = config_dir.join("providers.d").join(format!("{}.yaml", params.provider_name));

    let mut provider = get_provider_templates().get(&params.provider_name).cloned()
        .ok_or(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "Provider template not found".to_string()))?;

    let provider_content = tokio::fs::read_to_string(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error reading file: {}", e)))?;

    let provider_value = serde_yaml::from_str::<serde_yaml::Value>(&provider_content)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error parsing provider file: {}", e)))?;

    provider.apply_override(provider_value)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    add_running_models(&mut provider);
    populate_model_records(&mut provider);
    apply_models_dict_patch(&mut provider);
    add_name_and_id_to_model_records(&mut provider);
    tracing::info!("Provider models: {:#?}", provider);

    let result = serde_json::json!({
        "chat_models": provider.chat_models.into_iter()
            .map(|(_, model)| ModelLightResponse::new(model, false)).collect::<Vec<_>>(),
        "completion_models": provider.completion_models.into_iter()
            .map(|(_, model)| ModelLightResponse::new(model, false)).collect::<Vec<_>>(),
        "embedding_model": ModelLightResponse::new(provider.embedding_model, false),
    });
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&result).unwrap()))
        .unwrap())
}