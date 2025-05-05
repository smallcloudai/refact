use axum::extract::Query;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::ModelType;
use crate::caps::{ChatModelRecord, CompletionModelFamily, CompletionModelRecord, EmbeddingModelRecord, HasBaseModelRecord};
use crate::custom_error::{MapErrToString, ScratchError};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::caps::providers::{get_known_models, get_provider_from_server, get_provider_from_template_and_config_file, get_provider_model_default_settings_ui, get_provider_templates, read_providers_d, CapsProvider};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderDTO {
    name: String,
    endpoint_style: String,
    chat_endpoint: String,
    completion_endpoint: String,
    embedding_endpoint: String,
    api_key: String,
    #[serde(default)]
    tokenizer_api_key: String,

    chat_default_model: String,
    chat_thinking_model: String,
    chat_light_model: String,

    enabled: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default = "default_true")]
    supports_completion: bool,
}

fn default_true() -> bool { true }

impl ProviderDTO {
    pub fn from_caps_provider(provider: CapsProvider, readonly: bool) -> Self {
        ProviderDTO {
            name: provider.name,
            endpoint_style: provider.endpoint_style,
            chat_endpoint: provider.chat_endpoint,
            completion_endpoint: if provider.supports_completion { provider.completion_endpoint } else { String::new() },
            embedding_endpoint: provider.embedding_endpoint,
            api_key: provider.api_key,
            tokenizer_api_key: provider.tokenizer_api_key,
            chat_default_model: provider.defaults.chat_default_model,
            chat_light_model: provider.defaults.chat_light_model,
            chat_thinking_model: provider.defaults.chat_thinking_model,
            enabled: provider.enabled,
            readonly: readonly,
            supports_completion: provider.supports_completion,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ModelLightResponse {
    name: String,
    enabled: bool,
    removable: bool,
    user_configured: bool,
}

impl ModelLightResponse {
    pub fn new<T: HasBaseModelRecord>(model: T) -> Self {
        ModelLightResponse {
            name: model.base().name.clone(),
            enabled: model.base().enabled,
            removable: model.base().removable,
            user_configured: model.base().user_configured,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatModelDTO {
    n_ctx: usize,
    name: String,
    tokenizer: String,
    enabled: bool,

    supports_tools: bool,
    supports_multimodality: bool,
    supports_clicks: bool,
    supports_agent: bool,
    supports_reasoning: Option<String>,
    supports_boost_reasoning: bool,
    default_temperature: Option<f32>,

    #[serde(skip_deserializing, rename = "type", default = "model_type_chat")]
    model_type: ModelType,
}

fn model_type_chat() -> ModelType { ModelType::Chat }

impl ChatModelDTO {
    pub fn new(chat_model: ChatModelRecord) -> Self {
        ChatModelDTO {
            n_ctx: chat_model.base.n_ctx,
            name: chat_model.base.name,
            tokenizer: chat_model.base.tokenizer,
            enabled: chat_model.base.enabled,
            supports_tools: chat_model.supports_tools,
            supports_multimodality: chat_model.supports_multimodality,
            supports_clicks: chat_model.supports_clicks,
            supports_agent: chat_model.supports_agent,
            supports_reasoning: chat_model.supports_reasoning,
            supports_boost_reasoning: chat_model.supports_boost_reasoning,
            default_temperature: chat_model.default_temperature,
            model_type: ModelType::Chat,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompletionModelDTO {
    n_ctx: usize,
    name: String,
    enabled: bool,
    model_family: Option<CompletionModelFamily>,
    #[serde(skip_deserializing, rename = "type", default = "model_type_completion")]
    model_type: ModelType,
}

fn model_type_completion() -> ModelType { ModelType::Completion }

impl CompletionModelDTO {
    pub fn new(completion_model: CompletionModelRecord) -> Self {
        CompletionModelDTO {
            n_ctx: completion_model.base.n_ctx,
            name: completion_model.base.name,
            enabled: completion_model.base.enabled,
            model_family: completion_model.model_family,
            model_type: ModelType::Completion,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbeddingModelDTO {
    n_ctx: usize,
    name: String,
    tokenizer: String,
    enabled: bool,

    embedding_size: i32,
    rejection_threshold: f32,
    embedding_batch: usize,

    #[serde(skip_deserializing, rename = "type", default = "model_type_embedding")]
    model_type: ModelType,
}

fn model_type_embedding() -> ModelType { ModelType::Embedding }

impl EmbeddingModelDTO {
    pub fn new(embedding_model: EmbeddingModelRecord) -> Self {
        EmbeddingModelDTO {
            n_ctx: embedding_model.base.n_ctx,
            name: embedding_model.base.name,
            tokenizer: embedding_model.base.tokenizer,
            enabled: embedding_model.base.enabled,
            embedding_size: embedding_model.embedding_size,
            rejection_threshold: embedding_model.rejection_threshold,
            embedding_batch: embedding_model.embedding_batch,
            model_type: ModelType::Embedding,
        }
    }
}

pub async fn handle_v1_providers(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Response<Body> {
    let (config_dir, experimental) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.experimental)
    };

    let template_names = get_provider_templates().keys().collect::<Vec<_>>();
    let (providers, read_errors) = read_providers_d(Vec::new(), &config_dir, experimental).await;

    let mut result = providers.into_iter()
        .filter(|p| template_names.contains(&&p.name))
        .map(|p| json!({
            "name": p.name,
            "enabled": p.enabled,
            "readonly": false,
            "supports_completion": p.supports_completion
        }))
        .collect::<Vec<_>>();

    match crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            if !caps.cloud_name.is_empty() {
                result.retain(|p| p["name"] != caps.cloud_name);
                result.insert(0, json!({
                    "name": caps.cloud_name.clone(),
                    "enabled": true,
                    "readonly": true,
                    "supports_completion": true
                }));
            }
        },
        Err(e) => {
            tracing::error!("Failed to load caps, server provider will not be included: {}", e);
        }
    }

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
pub struct ProviderQueryParams {
    #[serde(rename = "provider-name")]
    provider_name: String,
}

pub async fn handle_v1_get_provider(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<ProviderQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let use_server_provider = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => !caps.cloud_name.is_empty() && caps.cloud_name == params.provider_name,
        Err(e) => {
            tracing::error!("Failed to load caps: {}", e);
            false
        }
    };

    let provider_dto = if use_server_provider {
        let provider = get_provider_from_server(gcx.clone()).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        ProviderDTO::from_caps_provider(provider, true)
    } else {
        let (config_dir, experimental) = {
            let gcx_locked = gcx.read().await;
            (gcx_locked.config_dir.clone(), gcx_locked.cmdline.experimental)
        };
        let provider = get_provider_from_template_and_config_file(&config_dir, &params.provider_name, false, true, experimental).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        ProviderDTO::from_caps_provider(provider, false)
    };

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
    update_yaml_field_if_needed(&mut file_value, "tokenizer_api_key",
        provider_dto.tokenizer_api_key, provider_template.tokenizer_api_key);
    update_yaml_field_if_needed(&mut file_value, "chat_endpoint",
        provider_dto.chat_endpoint, provider_template.chat_endpoint);
    update_yaml_field_if_needed(&mut file_value, "completion_endpoint",
        provider_dto.completion_endpoint, provider_template.completion_endpoint);
    update_yaml_field_if_needed(&mut file_value, "embedding_endpoint",
        provider_dto.embedding_endpoint, provider_template.embedding_endpoint);
    update_yaml_field_if_needed(&mut file_value, "chat_default_model",
        provider_dto.chat_default_model, provider_template.defaults.chat_default_model);
    update_yaml_field_if_needed(&mut file_value, "chat_light_model",
        provider_dto.chat_light_model, provider_template.defaults.chat_light_model);
    update_yaml_field_if_needed(&mut file_value, "chat_thinking_model",
        provider_dto.chat_thinking_model, provider_template.defaults.chat_thinking_model);
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

pub async fn handle_v1_delete_provider(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<ProviderQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let use_server_provider = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => !caps.cloud_name.is_empty() && caps.cloud_name == params.provider_name,
        Err(e) => {
            tracing::error!("Failed to load caps: {}", e);
            false
        }
    };

    if use_server_provider {
        return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot delete server provider".to_string()));
    }

    let config_dir = gcx.read().await.config_dir.clone();

    if !get_provider_templates().contains_key(&params.provider_name) {
        return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY,
            format!("Provider template '{}' not found", params.provider_name)));
    }

    let provider_path = config_dir.join("providers.d")
        .join(format!("{}.yaml", params.provider_name));

    if !provider_path.exists() {
        return Err(ScratchError::new(StatusCode::NOT_FOUND,
            format!("Provider '{}' does not exist", params.provider_name)));
    }

    tokio::fs::remove_file(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete provider file: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "success": true }).to_string()))
        .unwrap())
}

pub async fn handle_v1_models(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<ProviderQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let use_server_provider = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => !caps.cloud_name.is_empty() && caps.cloud_name == params.provider_name,
        Err(e) => {
            tracing::error!("Failed to load caps: {}", e);
            false
        }
    };

    let experimental = gcx.read().await.cmdline.experimental;
    let provider = if use_server_provider {
        get_provider_from_server(gcx.clone()).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?
    } else {
        let config_dir = gcx.read().await.config_dir.clone();
        get_provider_from_template_and_config_file(&config_dir, &params.provider_name, false, true, experimental).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?
    };

    let result = serde_json::json!({
        "chat_models": provider.chat_models.into_iter()
            .map(|(_, model)| ModelLightResponse::new(model)).collect::<Vec<_>>(),
        "completion_models": if provider.supports_completion {
            provider.completion_models.into_iter()
                .map(|(_, model)| ModelLightResponse::new(model)).collect::<Vec<_>>()
        } else {
            Vec::<ModelLightResponse>::new()
        },
        "embedding_model": ModelLightResponse::new(provider.embedding_model),
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&result).unwrap()))
        .unwrap())
}

#[derive(Deserialize)]
pub struct ModelQueryParams {
    model: Option<String>,
    provider: String,
    #[serde(rename = "type")]
    model_type: ModelType,
}

#[derive(Deserialize)]
pub struct ModelDefaultQueryParams {
    provider: String,
    #[serde(rename = "type")]
    model_type: ModelType,
}

pub async fn handle_v1_get_model(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<ModelQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let use_server_provider = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => !caps.cloud_name.is_empty() && caps.cloud_name == params.provider,
        Err(e) => {
            tracing::error!("Failed to load caps: {}", e);
            false
        }
    };

    let experimental = gcx.read().await.cmdline.experimental;
    let provider = if use_server_provider {
        get_provider_from_server(gcx.clone()).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?
    } else {
        let config_dir = gcx.read().await.config_dir.clone();
        get_provider_from_template_and_config_file(&config_dir, &params.provider, false, true, experimental).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?
    };

    let model = match params.model_type {
        ModelType::Chat => {
            let model_name = params.model.ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing `model` query parameter".to_string()))?;
            let chat_model = provider.chat_models.get(&model_name).cloned()
                .ok_or(ScratchError::new(StatusCode::NOT_FOUND, format!("Chat model {} not found for provider {}", model_name, params.provider)))?;
            serde_json::json!(ChatModelDTO::new(chat_model))
        },
        ModelType::Completion => {
            if !provider.supports_completion {
                return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Provider {} does not support completion", params.provider)));
            }
            let model_name = params.model.ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing `model` query parameter".to_string()))?;
            let completion_model = provider.completion_models.get(&model_name).cloned()
                .ok_or(ScratchError::new(StatusCode::NOT_FOUND, format!("Completion model {} not found for provider {}", model_name, params.provider)))?;
            serde_json::json!(CompletionModelDTO::new(completion_model))
        },
        ModelType::Embedding => {
            serde_json::json!(EmbeddingModelDTO::new(provider.embedding_model))
        },
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&model).unwrap()))
        .unwrap())
}

#[derive(Deserialize)]
pub struct ModelPOST {
    pub provider: String,
    pub model: serde_json::Value,
    #[serde(rename = "type")]
    pub model_type: ModelType,
}

pub async fn handle_v1_post_model(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<ModelPOST>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error parsing json: {}", e)))?;

    let config_dir = gcx.read().await.config_dir.clone();
    let provider_path = config_dir.join("providers.d").join(format!("{}.yaml", post.provider));

    let _provider_template = get_provider_templates().get(&post.provider)
        .ok_or(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "Provider template not found".to_string()))?;

    let mut file_value = read_yaml_file_as_value_if_exists(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    fn get_or_create_model_mapping(file_value: &mut serde_yaml::Value, models_key: &str, model_name: &str) -> serde_yaml::Mapping {
        if !file_value.get(models_key).is_some() {
            file_value[models_key] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }

        let model_entry = if file_value[models_key].get(model_name).is_some() {
            file_value[models_key][model_name].clone()
        } else {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        };

        model_entry.as_mapping().unwrap_or(&serde_yaml::Mapping::new()).clone()
    }

    match post.model_type {
        ModelType::Chat => {
            let chat_model = serde_json::from_value::<ChatModelDTO>(post.model)
                .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error parsing model: {}", e)))?;
            let models_key = "chat_models";

            let mut model_value = get_or_create_model_mapping(&mut file_value, models_key, &chat_model.name);

            model_value.insert("n_ctx".into(), chat_model.n_ctx.into());
            model_value.insert("tokenizer".into(), chat_model.tokenizer.into());
            model_value.insert("enabled".into(), chat_model.enabled.into());

            model_value.insert("supports_tools".into(), chat_model.supports_tools.into());
            model_value.insert("supports_multimodality".into(), chat_model.supports_multimodality.into());
            model_value.insert("supports_clicks".into(), chat_model.supports_clicks.into());
            model_value.insert("supports_agent".into(), chat_model.supports_agent.into());
            model_value.insert("supports_boost_reasoning".into(), chat_model.supports_boost_reasoning.into());

            model_value.insert("supports_reasoning".into(),
                match chat_model.supports_reasoning {
                    Some(supports_reasoning) => supports_reasoning.into(),
                    None => serde_yaml::Value::Null,
                }
            );
            model_value.insert("default_temperature".into(),
                match chat_model.default_temperature {
                    Some(default_temperature) => serde_yaml::Value::Number(serde_yaml::Number::from(default_temperature as f64)),
                    None => serde_yaml::Value::Null,
                }
            );

            file_value[models_key][chat_model.name] = model_value.into();
        },
        ModelType::Completion => {
            let completion_model = serde_json::from_value::<CompletionModelDTO>(post.model)
                .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error parsing model: {}", e)))?;
            let models_key = "completion_models";

            let mut model_value = get_or_create_model_mapping(&mut file_value, models_key, &completion_model.name);

            if let Some(model_family) = completion_model.model_family {
                let family_model_rec = get_known_models().completion_models.get(&model_family.to_string())
                    .expect(&format!("Model family {} not found in known models", model_family.to_string()));

                model_value.insert("model_family".into(), model_family.to_string().into());
                model_value.insert("scratchpad".into(), family_model_rec.scratchpad.clone().into());
                model_value.insert("scratchpad_patch".into(), serde_yaml::from_str(&family_model_rec.scratchpad_patch.to_string()).unwrap());
                model_value.insert("tokenizer".into(), family_model_rec.base.tokenizer.clone().into());
            }

            model_value.insert("n_ctx".into(), completion_model.n_ctx.into());
            model_value.insert("enabled".into(), completion_model.enabled.into());

            file_value[models_key][completion_model.name] = model_value.into();
        },
        ModelType::Embedding => {
            let embedding_model = serde_json::from_value::<EmbeddingModelDTO>(post.model)
                .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Error parsing model: {}", e)))?;
            let mut model_value = serde_yaml::Mapping::new();

            model_value.insert("n_ctx".into(), embedding_model.n_ctx.into());
            model_value.insert("name".into(), embedding_model.name.clone().into());
            model_value.insert("tokenizer".into(), embedding_model.tokenizer.into());
            model_value.insert("enabled".into(), embedding_model.enabled.into());

            model_value.insert("embedding_size".into(), embedding_model.embedding_size.into());
            model_value.insert("rejection_threshold".into(), serde_yaml::Value::Number(serde_yaml::Number::from(embedding_model.rejection_threshold as f64)));
            model_value.insert("embedding_batch".into(), embedding_model.embedding_batch.into());

            file_value["embedding_model"] = model_value.into();
        },
    }

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

pub async fn handle_v1_delete_model(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Query(params): Query<ModelQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let config_dir = gcx.read().await.config_dir.clone();
    let provider_path = config_dir.join("providers.d").join(format!("{}.yaml", params.provider));

    let _provider_template = get_provider_templates().get(&params.provider)
        .ok_or(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "Provider template not found".to_string()))?;

    let mut file_value = read_yaml_file_as_value_if_exists(&provider_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    match params.model_type {
        ModelType::Chat => {
            let model_name = params.model.as_ref()
                .ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing `model` query parameter".to_string()))?;
            let models_key = "chat_models";

            if !file_value.get(models_key).is_some() || !file_value[models_key].get(model_name).is_some() {
                return Err(ScratchError::new(StatusCode::NOT_FOUND,
                    format!("Chat model {} not found for provider {}", model_name, params.provider)));
            }

            if let Some(mapping) = file_value[models_key].as_mapping_mut() {
                mapping.remove(model_name);
            }
        },
        ModelType::Completion => {
            let model_name = params.model.as_ref()
                .ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing `model` query parameter".to_string()))?;
            let models_key = "completion_models";

            if !file_value.get(models_key).is_some() || !file_value[models_key].get(model_name).is_some() {
                return Err(ScratchError::new(StatusCode::NOT_FOUND,
                    format!("Completion model {} not found for provider {}", model_name, params.provider)));
            }

            if let Some(mapping) = file_value[models_key].as_mapping_mut() {
                mapping.remove(model_name);
            }
        },
        ModelType::Embedding => {
            if !file_value.get("embedding_model").is_some() {
                return Err(ScratchError::new(StatusCode::NOT_FOUND,
                    format!("Embedding model not found for provider {}", params.provider)));
            }

            file_value.as_mapping_mut().unwrap().remove("embedding_model");
        },
    }

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

pub async fn handle_v1_model_default(
    Query(params): Query<ModelDefaultQueryParams>,
) -> Result<Response<Body>, ScratchError> {
    let model_defaults = get_provider_model_default_settings_ui().get(&params.provider).ok_or_else(||
        ScratchError::new(StatusCode::NOT_FOUND, "Provider not found".to_string())
    )?;

    let response_json = match params.model_type {
        ModelType::Chat => serde_json::json!(ChatModelDTO::new(model_defaults.chat.clone())),
        ModelType::Completion => serde_json::json!(CompletionModelDTO::new(model_defaults.completion.clone())),
        ModelType::Embedding => serde_json::json!(EmbeddingModelDTO::new(model_defaults.embedding.clone())),
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
        .unwrap())
}

pub async fn handle_v1_completion_model_families() -> Response<Body> {
    let response_json = json!({
        "model_families": CompletionModelFamily::all_variants()
            .into_iter().map(|family| family.to_string()).collect::<Vec<_>>()
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
        .unwrap()
}