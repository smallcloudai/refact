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
use crate::caps::providers::{add_models_to_caps, read_providers_d, resolve_provider_api_key,
    post_process_provider, CapsProvider};
use crate::caps::self_hosted::SelfHostedCaps;

pub const CAPS_FILENAME: &str = "refact-caps";
pub const CAPS_FILENAME_FALLBACK: &str = "coding_assistant_caps.json";

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
    #[serde(default, skip_serializing)]
    pub tokenizer_api_key: String,

    #[serde(default, skip_serializing)]
    pub support_metadata: bool,
    #[serde(default, skip_serializing)]
    pub similar_models: Vec<String>,
    #[serde(default)]
    pub tokenizer: String,

    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub experimental: bool,
    // Fields used for Config/UI management
    #[serde(skip_deserializing)]
    pub removable: bool,
    #[serde(skip_deserializing)]
    pub user_configured: bool,
}

fn default_true() -> bool { true }

pub trait HasBaseModelRecord {
    fn base(&self) -> &BaseModelRecord;
    fn base_mut(&mut self) -> &mut BaseModelRecord;
}

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct ChatModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    #[serde(default = "default_chat_scratchpad", skip_serializing)]
    pub scratchpad: String,
    #[serde(default, skip_serializing)]
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

pub fn default_chat_scratchpad() -> String { "PASSTHROUGH".to_string() }

impl HasBaseModelRecord for ChatModelRecord {
    fn base(&self) -> &BaseModelRecord { &self.base }
    fn base_mut(&mut self) -> &mut BaseModelRecord { &mut self.base }
}

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct CompletionModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    #[serde(default = "default_completion_scratchpad")]
    pub scratchpad: String,
    #[serde(default = "default_completion_scratchpad_patch")]
    pub scratchpad_patch: serde_json::Value,

    pub model_family: Option<CompletionModelFamily>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionModelFamily {
    #[serde(rename = "qwen2.5-coder-base")]
    Qwen2_5CoderBase,
    #[serde(rename = "starcoder")]
    Starcoder,
    #[serde(rename = "deepseek-coder")]
    DeepseekCoder,
}

impl CompletionModelFamily {
    pub fn to_string(self) -> String {
        serde_json::to_value(self).ok()
            .and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default()
    }

    pub fn all_variants() -> Vec<CompletionModelFamily> {
        vec![
            CompletionModelFamily::Qwen2_5CoderBase,
            CompletionModelFamily::Starcoder,
            CompletionModelFamily::DeepseekCoder,
        ]
    }
}

pub fn default_completion_scratchpad() -> String { "REPLACE_PASSTHROUGH".to_string() }

pub fn default_completion_scratchpad_patch() -> serde_json::Value { serde_json::json!({
    "context_format": "chat",
    "rag_ratio": 0.5
}) }

impl HasBaseModelRecord for CompletionModelRecord {
    fn base(&self) -> &BaseModelRecord { &self.base }
    fn base_mut(&mut self) -> &mut BaseModelRecord { &mut self.base }
}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
pub struct EmbeddingModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    pub embedding_size: i32,
    pub rejection_threshold: f32,
    pub embedding_batch: usize,
}

pub fn default_rejection_threshold() -> f32 { 0.63 }

pub fn default_embedding_batch() -> usize { 64 }

impl HasBaseModelRecord for EmbeddingModelRecord {
    fn base(&self) -> &BaseModelRecord { &self.base }
    fn base_mut(&mut self) -> &mut BaseModelRecord { &mut self.base }
}

impl EmbeddingModelRecord {
    pub fn is_configured(&self) -> bool {
        !self.base.name.is_empty() && (self.embedding_size > 0 || self.embedding_batch > 0 || self.base.n_ctx > 0)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CapsMetadata {
    pub pricing: serde_json::Value,
    pub features: Vec<String>
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantCaps {
    #[serde(deserialize_with = "normalize_string")]
    pub cloud_name: String, // "refact" or "refact_self_hosted"

    #[serde(default = "default_telemetry_basic_dest")]
    pub telemetry_basic_dest: String,
    #[serde(default = "default_telemetry_retrieve_my_own")]
    pub telemetry_basic_retrieve_my_own: String,

    #[serde(skip_deserializing)]
    pub completion_models: IndexMap<String, Arc<CompletionModelRecord>>, // keys are "provider/model"
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

    #[serde(default = "default_hf_tokenizer_template")]
    pub hf_tokenizer_template: String,  // template for HuggingFace tokenizer URLs

    #[serde(default)]  // Need for metadata from cloud, e.g. pricing for models; used only in chat-js
    pub metadata: CapsMetadata
}

fn default_telemetry_retrieve_my_own() -> String {
    "https://www.smallcloud.ai/v1/telemetry-retrieve-my-own-stats".to_string()
}

pub fn default_hf_tokenizer_template() -> String {
    "https://huggingface.co/$HF_MODEL/resolve/main/tokenizer.json".to_string()
}

fn default_telemetry_basic_dest() -> String {
    "https://www.smallcloud.ai/v1/telemetry-basic".to_string()
}

pub fn normalize_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let s: String = String::deserialize(deserializer)?;
    Ok(s.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect())
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

impl DefaultModels {
    pub fn apply_override(&mut self, other: &DefaultModels, provider_name: Option<&str>) {
        if !other.completion_default_model.is_empty() {
            self.completion_default_model = match provider_name {
                Some(provider) => format!("{}/{}", provider, other.completion_default_model),
                None => other.completion_default_model.clone(),
            };
        }
        if !other.chat_default_model.is_empty() {
            self.chat_default_model = match provider_name {
                Some(provider) => format!("{}/{}", provider, other.chat_default_model),
                None => other.chat_default_model.clone(),
            };
        }
        if !other.chat_thinking_model.is_empty() {
            self.chat_thinking_model = match provider_name {
                Some(provider) => format!("{}/{}", provider, other.chat_thinking_model),
                None => other.chat_thinking_model.clone(),
            };
        }
        if !other.chat_light_model.is_empty() {
            self.chat_light_model = match provider_name {
                Some(provider) => format!("{}/{}", provider, other.chat_light_model),
                None => other.chat_light_model.clone(),
            };
        }
    }
}

pub async fn load_caps_value_from_url(
    cmdline: CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(serde_json::Value, String), String> {
    let caps_urls = if cmdline.address_url.to_lowercase() == "refact" {
        vec!["https://app.refact.ai/coding_assistant_caps.json".to_string()]
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
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_str(&format!("refact-lsp {}", crate::version::build::PKG_VERSION)).unwrap());
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

    if let Some(json_value) = last_response_json {
        if let Some(detail) = json_value.get("detail").and_then(|d| d.as_str()) {
            return Err(detail.to_string());
        }
    }

    Err(format!("cannot fetch caps, status={}", last_status))
}

pub async fn load_caps(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<Arc<CodeAssistantCaps>, String> {
    let (config_dir, cmdline_api_key, experimental) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.api_key.clone(), gcx_locked.cmdline.experimental)
    };

    let (caps_value, caps_url) = load_caps_value_from_url(cmdline, gcx).await?;

    let (mut caps, server_providers) = match serde_json::from_value::<SelfHostedCaps>(caps_value.clone()) {
        Ok(self_hosted_caps) => (self_hosted_caps.into_caps(&caps_url, &cmdline_api_key)?, Vec::new()),
        Err(_) => {
            let caps = serde_json::from_value::<CodeAssistantCaps>(caps_value.clone())
                .map_err_with_prefix("Failed to parse caps:")?;
            let mut server_provider = serde_json::from_value::<CapsProvider>(caps_value)
                .map_err_with_prefix("Failed to parse caps provider:")?;
            resolve_relative_urls(&mut server_provider, &caps_url)?;
            (caps, vec![server_provider])
        }
    };

    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;

    let (mut providers, error_log) = read_providers_d(server_providers, &config_dir, experimental).await;
    providers.retain(|p| p.enabled);
    for e in error_log {
        tracing::error!("{e}");
    }
    for provider in &mut providers {
        post_process_provider(provider, false, experimental);
        provider.api_key = resolve_provider_api_key(&provider, &cmdline_api_key);
    }
    add_models_to_caps(&mut caps, providers);

    Ok(Arc::new(caps))
}

pub fn resolve_relative_urls(provider: &mut CapsProvider, caps_url: &str) -> Result<(), String> {
    provider.chat_endpoint = relative_to_full_url(caps_url, &provider.chat_endpoint)?;
    provider.completion_endpoint = relative_to_full_url(caps_url, &provider.completion_endpoint)?;
    provider.embedding_endpoint = relative_to_full_url(caps_url, &provider.embedding_endpoint)?;
    Ok(())
}

pub fn strip_model_from_finetune(model: &str) -> String {
    model.split(":").next().unwrap().to_string()
}

pub fn relative_to_full_url(
    caps_url: &str,
    maybe_relative_url: &str,
) -> Result<String, String> {
    if maybe_relative_url.starts_with("http") {
        Ok(maybe_relative_url.to_string())
    } else if maybe_relative_url.is_empty() {
        Ok("".to_string())
    } else {
        let base_url = Url::parse(caps_url)
            .map_err(|_| format!("failed to parse caps url: {}", caps_url))?;
        let joined_url = base_url.join(maybe_relative_url)
            .map_err(|_| format!("failed to join url: {}", maybe_relative_url))?;
        Ok(joined_url.to_string())
    }
}

pub fn resolve_model<'a, T>(
    models: &'a IndexMap<String, Arc<T>>,
    model_id: &str,
) -> Result<Arc<T>, String> {
    models.get(model_id).or_else(
        || models.get(&strip_model_from_finetune(model_id))
    ).cloned().ok_or(format!("Model '{}' not found. Server has the following models: {:?}", model_id, models.keys()))
}

pub fn resolve_completion_model<'a>(
    caps: Arc<CodeAssistantCaps>,
    requested_model_id: &str,
    try_refact_fallbacks: bool,
) -> Result<Arc<CompletionModelRecord>, String> {
    let model_id = if !requested_model_id.is_empty() {
        requested_model_id
    } else {
        &caps.defaults.completion_default_model
    };

    match resolve_model(&caps.completion_models, model_id) {
        Ok(model) => Ok(model),
        Err(first_err) if try_refact_fallbacks => {
            if let Ok(model) = resolve_model(&caps.completion_models, &format!("refact/{model_id}")) {
                return Ok(model);
            }
            if let Ok(model) = resolve_model(&caps.completion_models, &format!("refact_self_hosted/{model_id}")) {
                return Ok(model);
            }
            Err(first_err)
        }
        Err(err) => Err(err),
    }
}
