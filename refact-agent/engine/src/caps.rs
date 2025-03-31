use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock as ARwLock;
use url::Url;
use tracing::{info, warn};

use crate::custom_error::MapErrToString;
use crate::global_context::CommandLine;
use crate::global_context::GlobalContext;
use crate::providers::{add_models_to_caps, apply_models_dict_patch, populate_provider_model_records, 
    read_providers_d, resolve_provider_api_key, CapsProvider};

const CAPS_FILENAME: &str = "refact-caps";
const CAPS_FILENAME_FALLBACK: &str = "coding_assistant_caps.json";

#[derive(Debug, Serialize, Clone, Deserialize, Default, PartialEq)]
pub struct BaseModelRecord {
    #[serde(default)]
    pub n_ctx: usize,

    /// Actual model name, e.g. "gpt-4o"
    #[serde(default)]
    pub name: String, 
    /// provider/model_name, e.g. "openai/gpt-4o"
    #[serde(skip_deserializing)]
    pub id: String, 

    #[serde(default, skip_serializing)]
    pub endpoint: String,
    #[serde(default, skip_serializing)]
    pub endpoint_style: String,
    #[serde(default, skip_serializing)]
    pub api_key: String,

    #[serde(default)]
    pub support_metadata: bool,
    #[serde(default, skip_serializing)]
    pub similar_models: Vec<String>,
    #[serde(default, skip_serializing)]
    pub tokenizer: String,
}

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct ChatModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    #[serde(default = "default_chat_scratchpad")]
    pub scratchpad: String,
    #[serde(default)]
    pub scratchpad_patch: serde_json::Value,

    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_multimodality: bool,
    #[serde(default)]
    pub supports_clicks: bool,
    #[serde(default)]
    pub supports_agent: bool,
    #[serde(default)]
    pub supports_reasoning: Option<String>,
    #[serde(default)]
    pub supports_boost_reasoning: bool,
    #[serde(default)]
    pub default_temperature: Option<f32>,
}

fn default_chat_scratchpad() -> String { "PASSTHROUGH".to_string() }

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct CompletionModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    pub scratchpad: String,
    #[serde(default)]
    pub scratchpad_patch: serde_json::Value,
}

#[derive(Debug, Serialize, Clone, Deserialize, Default, PartialEq)]
pub struct EmbeddingModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    pub embedding_size: i32,
    #[serde(default = "default_rejection_threshold")]
    pub rejection_threshold: f32,
    #[serde(default)]
    pub embedding_batch: usize,
}

fn default_rejection_threshold() -> f32 { 0.63 }

impl EmbeddingModelRecord {
    pub fn is_configured(&self) -> bool {
        !self.base.name.is_empty() && (self.embedding_size > 0 || self.embedding_batch > 0 || self.base.n_ctx > 0)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantCaps {
    #[serde(default = "default_telemetry_basic_dest")]
    pub telemetry_basic_dest: String,
    #[serde(default = "default_telemetry_retrieve_my_own")]
    pub telemetry_basic_retrieve_my_own: String,

    #[serde(skip_deserializing)]
    pub completion_models: IndexMap<String, Arc<CompletionModelRecord>>,
    #[serde(skip_deserializing)]
    pub chat_models: IndexMap<String, Arc<ChatModelRecord>>,
    #[serde(skip_deserializing)]
    pub embedding_model: EmbeddingModelRecord,

    #[serde(flatten, skip_deserializing)]
    pub defaults: DefaultModels,
    
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes

    #[serde(default)]
    pub customization: String,  // on self-hosting server, allows to customize yaml_configs & friends for all engineers
}

fn default_telemetry_retrieve_my_own() -> String { 
    "https://www.smallcloud.ai/v1/telemetry-retrieve-my-own-stats".to_string() 
}

fn default_telemetry_basic_dest() -> String { 
    "https://www.smallcloud.ai/v1/telemetry-basic".to_string() 
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DefaultModels {
    #[serde(default, alias = "code_completion_default_model", alias = "completion_model")]
    pub completion_default_model: String,
    #[serde(default, alias = "code_chat_default_model", alias = "chat_model")]
    pub chat_default_model: String,
    #[serde(default)]
    pub chat_thinking_model: String,
    #[serde(default)]
    pub chat_light_model: String,
}

async fn load_caps_value_from_url(
    cmdline: CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(serde_json::Value, String), String> {
    let caps_urls = if cmdline.address_url.to_lowercase() == "refact" {
        vec!["https://inference.smallcloud.ai/coding_assistant_caps.json".to_string()]
    } else {
        let base_url = Url::parse(&cmdline.address_url)
            .map_err(|_| "failed to parse address url".to_string())?;
            
        vec![
            base_url.join(&CAPS_FILENAME).map_err(|_| "failed to join caps URL".to_string())?.to_string(),
            base_url.join(&CAPS_FILENAME_FALLBACK).map_err(|_| "failed to join fallback caps URL".to_string())?.to_string(),
        ]
    };

    let http_client = gcx.read().await.http_client.clone();
    let mut headers = reqwest::header::HeaderMap::new();
    
    if !cmdline.api_key.is_empty() {
        headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(&format!("Bearer {}", cmdline.api_key)).unwrap());
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_str(&format!("refact-lsp {}", crate::version::build_info::PKG_VERSION)).unwrap());
    }

    let mut last_status = 0;
    let mut last_response_json: Option<serde_json::Value> = None;

    for url in &caps_urls {
        info!("fetching caps from {}", url);
        let response = http_client.get(url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        last_status = response.status().as_u16();
        
        if let Ok(json_value) = response.json::<serde_json::Value>().await {
            if last_status == 200 {
                return Ok((json_value, url.clone()));
            }
            last_response_json = Some(json_value.clone());
            warn!("status={}; server responded with:\n{}", last_status, json_value);
        }
    }

    if let Some(json) = last_response_json {
        if let Some(detail) = json.get("detail").and_then(|d| d.as_str()) {
            return Err(detail.to_string());
        }
    }
    
    Err(format!("cannot fetch caps, status={}", last_status))
}

pub async fn load_caps(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<Arc<CodeAssistantCaps>, String> {
    let (config_dir, cmdline_api_key) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.api_key.clone())
    };
    
    let (caps_value, caps_url) = load_caps_value_from_url(cmdline, gcx).await?;
    
    let mut caps = serde_json::from_value::<CodeAssistantCaps>(caps_value.clone())
        .map_err_with_prefix("Failed to parse caps:")?;
    let mut server_provider = serde_json::from_value::<CapsProvider>(caps_value)
        .map_err_with_prefix("Failed to parse caps provider:")?;

    server_provider.chat_endpoint = relative_to_full_url(&caps_url, &server_provider.chat_endpoint)?;
    server_provider.completion_endpoint = relative_to_full_url(&caps_url, &server_provider.completion_endpoint)?;
    server_provider.embedding_endpoint = relative_to_full_url(&caps_url, &server_provider.embedding_endpoint)?;
    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;
    
    let (mut providers, error_log) = read_providers_d(vec![server_provider], &config_dir).await;
    for e in error_log {
        tracing::error!("{e}");
    }
    populate_provider_model_records(&mut providers)?;
    apply_models_dict_patch(&mut providers);
    for provider in &mut providers {
        provider.api_key = resolve_provider_api_key(&provider, &cmdline_api_key);
    }
    add_models_to_caps(&mut caps, providers);
    Ok(Arc::new(caps))
}

pub fn strip_model_from_finetune(model: &str) -> String {
    model.split(":").next().unwrap().to_string()
}

fn relative_to_full_url(
    caps_url: &str,
    maybe_relative_url: &str,
) -> Result<String, String> {
    if maybe_relative_url.starts_with("http") {
        Ok(maybe_relative_url.to_string())
    } else if maybe_relative_url.is_empty() {
        Ok("".to_string())
    } else {
        let base_url = Url::parse(caps_url).map_err(|_| "failed to parse address url (3)".to_string())?;
        let joined_url = base_url.join(maybe_relative_url).map_err(|_| "failed to join URL \"{}\" and possibly relative \"{}\"".to_string())?;
        Ok(joined_url.to_string())
    }
}

pub fn resolve_model<'a, T>(
    models: &'a IndexMap<String, Arc<T>>,
    requested_model_id: &str,
    default_model_id: &str,
) -> Result<Arc<T>, String> {
    let model_id = if !requested_model_id.is_empty() {
        requested_model_id
    } else {
        default_model_id
    };
    models.get(model_id).or_else(
        || models.get(&strip_model_from_finetune(model_id))
    ).cloned().ok_or(format!("Model '{}' not found. Server has the following models: {:?}", model_id, models.keys()))
}

pub fn resolve_chat_model<'a>(
    caps:  Arc<CodeAssistantCaps>,
    requested_model_id: &str,
) -> Result<Arc<ChatModelRecord>, String> {
    resolve_model(
        &caps.chat_models, 
        requested_model_id, 
        &caps.defaults.chat_default_model
    )
}

pub fn resolve_completion_model<'a>(
    caps: Arc<CodeAssistantCaps>,
    requested_model_id: &str,
) -> Result<Arc<CompletionModelRecord>, String> {
    resolve_model(
        &caps.completion_models, 
        requested_model_id, 
        &caps.defaults.completion_default_model
    )
}