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
use crate::providers::add_name_and_id_to_model_records;
use crate::providers::add_running_models;
use crate::providers::populate_model_records;
use crate::providers::{add_models_to_caps, apply_models_dict_patch, 
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
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip_deserializing)]
    pub removable: bool,

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

impl HasBaseModelRecord for ChatModelRecord {
    fn base(&self) -> &BaseModelRecord { &self.base }
    fn base_mut(&mut self) -> &mut BaseModelRecord { &mut self.base }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CapsV2EmbeddingModelRecord {
    #[serde(default)]
    pub n_ctx: usize,
    #[serde(default)]
    pub size: i32,
}


fn default_chat_scratchpad() -> String { "PASSTHROUGH".to_string() }

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct CompletionModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,

    #[serde(default = "default_completion_scratchpad")]
    pub scratchpad: String,
    #[serde(default = "default_completion_scratchpad_patch")]
    pub scratchpad_patch: serde_json::Value,
}

fn default_completion_scratchpad() -> String { "REPLACE_PASSTHROUGH".to_string() }

fn default_completion_scratchpad_patch() -> serde_json::Value { serde_json::json!({
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
pub struct CodeAssistantCaps {
    pub cloud_name: String,

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
    
    #[serde(default = "default_hf_tokenizer_template")]
    pub hf_tokenizer_template: String,  // template for HuggingFace tokenizer URLs
}

fn default_telemetry_retrieve_my_own() -> String { 
    "https://www.smallcloud.ai/v1/telemetry-retrieve-my-own-stats".to_string() 
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CodeAssistantCapsV2Completion {
    pub endpoint: String,
    pub models: IndexMap<String, ModelRecord>,
    pub default_model: String,
    pub default_multiline_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CodeAssistantCapsV2Chat {
    pub endpoint: String,
    pub models: IndexMap<String, ModelRecord>,
    pub default_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CodeAssistantCapsV2Embedding {
    pub endpoint: String,
    pub models: IndexMap<String, EmbeddingModelRecord>,
    pub default_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CodeAssistantCapsV2TelemetryEndpoints {
    pub telemetry_basic_endpoint: String,
    pub telemetry_corrected_snippets_endpoint: String,
    pub telemetry_basic_retrieve_my_own_endpoint: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CodeAssistantCapsV2 {
    pub cloud_name: String,

    pub completion: CodeAssistantCapsV2Completion,
    pub chat: CodeAssistantCapsV2Chat,
    pub embedding: CodeAssistantCapsV2Embedding,

    pub telemetry_endpoints: CodeAssistantCapsV2TelemetryEndpoints,
    pub tokenizer_endpoints: HashMap<String, String>,

    #[serde(default)]
    pub customization: String,
    #[serde(default)]
    pub default_system_prompt: String,

    pub caps_version: i64,
}

pub fn default_hf_tokenizer_template() -> String {
    "https://huggingface.co/$HF_MODEL/resolve/main/tokenizer.json".to_string()
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
        vec!["https://inference.smallcloud.ai/coding_assistant_caps.json".to_string()]
    } else {
        let base_url = Url::parse(&cmdline.address_url)
            .map_err(|_| "failed to parse address url".to_string())?;
            
        vec![
            base_url.join(&CAPS_FILENAME).map_err(|_| "failed to join caps URL".to_string())?.to_string(),
            base_url.join(&CAPS_FILENAME_FALLBACK).map_err(|_| "failed to join fallback caps URL".to_string())?.to_string(),
        ]
    };
}

fn load_caps_from_buf_v2(
    buffer: &String,
    caps_url: &String,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    // Try to parse as V2 format
    let caps_v2: CodeAssistantCapsV2 = match serde_json::from_str(buffer) {
        Ok(v) => v,
        Err(_) => return Err("failed to load in v2 format".to_string()),
    };

    // Convert V2 to V1 format
    let mut caps = CodeAssistantCaps {
        cloud_name: caps_v2.cloud_name,
        endpoint_style: "openai".to_string(),
        chat_endpoint_style: "openai".to_string(),
        completion_endpoint_style: "openai".to_string(),
        endpoint_embeddings_style: "openai".to_string(),

        // Completion related fields
        completion_endpoint: relative_to_full_url(&caps_url, &caps_v2.completion.endpoint)?,
        code_completion_models: caps_v2.completion.models.clone(),
        code_completion_default_model: caps_v2.completion.default_model.clone(),
        multiline_code_completion_default_model: caps_v2.completion.default_multiline_model.clone(),

        // Chat related fields
        chat_endpoint: relative_to_full_url(&caps_url, &caps_v2.completion.endpoint)?,  // for completion-based chat
        endpoint_chat_passthrough: relative_to_full_url(&caps_url, &caps_v2.chat.endpoint)?,
        code_chat_models: caps_v2.chat.models.clone(),
        code_chat_default_model: caps_v2.chat.default_model.clone(),

        // Embeddings related fields
        endpoint_embeddings_template: relative_to_full_url(&caps_url, &caps_v2.embedding.endpoint)?,
        embedding_model: caps_v2.embedding.default_model.clone(),
        embedding_n_ctx: caps_v2.embedding.models.get(&caps_v2.embedding.default_model).cloned().unwrap_or_default().n_ctx,
        embedding_size: caps_v2.embedding.models.get(&caps_v2.embedding.default_model).cloned().unwrap_or_default().size,

        // Telemetry endpoints
        telemetry_basic_dest: relative_to_full_url(&caps_url, &caps_v2.telemetry_endpoints.telemetry_basic_endpoint)?,
        telemetry_basic_retrieve_my_own: relative_to_full_url(&caps_url, &caps_v2.telemetry_endpoints.telemetry_basic_retrieve_my_own_endpoint)?,

        tokenizer_path_template: "".to_string(),
        tokenizer_rewrite_path: {
            let mut rewritten_paths = HashMap::new();
            for (key, endpoint) in caps_v2.tokenizer_endpoints {
                let full_url = relative_to_full_url(&caps_url, &endpoint)?;
                rewritten_paths.insert(key, full_url);
            }
            rewritten_paths
        },

        // Version
        caps_version: caps_v2.caps_version,

        // Collect all models from completion and chat sections
        running_models: {
            let mut models = std::collections::HashSet::new();
            models.extend(caps_v2.completion.models.keys().cloned());
            models.extend(caps_v2.chat.models.keys().cloned());
            // models.extend(caps_v2.embedding.models.keys().cloned());
            models.into_iter().collect()
        },

        customization: caps_v2.customization.clone(),
        code_chat_default_system_prompt: caps_v2.default_system_prompt.clone(),

        ..Default::default()
    };

    // Convert relative URLs to absolute URLs
    caps.endpoint_embeddings_template = relative_to_full_url(&caps_url, &caps.endpoint_embeddings_template)?;
    caps.chat_endpoint = relative_to_full_url(&caps_url, &caps.chat_endpoint)?;
    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;

    // Set default embedding context size if not set
    if caps.embedding_n_ctx == 0 {
        caps.embedding_n_ctx = 512;
    }

    Ok(Arc::new(StdRwLock::new(caps)))
}

macro_rules! get_api_key_macro {
    ($gcx:expr, $caps:expr, $field:ident) => {{
        let cx_locked = $gcx.read().await;
        let custom_apikey = $caps.read().unwrap().$field.clone();
        if custom_apikey.is_empty() {
            cx_locked.cmdline.api_key.clone()
        } else if custom_apikey.starts_with("$") {
            let env_var_name = &custom_apikey[1..];
            match std::env::var(env_var_name) {
                Ok(env_value) => env_value,
                Err(e) => {
                    error!("tried to read API key from env var {}, but failed: {}\nTry editing ~/.config/refact/bring-your-own-key.yaml", env_var_name, e);
                    cx_locked.cmdline.api_key.clone()
                }
            }
        } else {
            custom_apikey
        }
    }};
}

pub async fn get_api_key(
    gcx: Arc<ARwLock<GlobalContext>>,
    use_this_fall_back_to_default_if_empty: String,
) -> String {
    let gcx_locked = gcx.write().await;
    if use_this_fall_back_to_default_if_empty.is_empty() {
        gcx_locked.cmdline.api_key.clone()
    } else if use_this_fall_back_to_default_if_empty.starts_with("$") {
        let env_var_name = &use_this_fall_back_to_default_if_empty[1..];
        match std::env::var(env_var_name) {
            Ok(env_value) => env_value,
            Err(e) => {
                error!("tried to read API key from env var {}, but failed: {}\nTry editing ~/.config/refact/bring-your-own-key.yaml", env_var_name, e);
                gcx_locked.cmdline.api_key.clone()
            }
        }
    } else {
        use_this_fall_back_to_default_if_empty
    }
}

#[allow(dead_code)]
async fn get_custom_chat_api_key(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, ScratchError> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    Ok(get_api_key_macro!(gcx, caps, chat_apikey))
}

#[cfg(feature="vecdb")]
pub async fn get_custom_embedding_api_key(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, ScratchError> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    Ok(get_api_key_macro!(gcx, caps, embedding_apikey))
}

#[allow(dead_code)]
async fn get_custom_completion_api_key(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, ScratchError> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    Ok(get_api_key_macro!(gcx, caps, completion_apikey))
}


async fn load_caps_buf_from_file(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(serde_json::Value, String), String> {
    let caps_urls = if cmdline.address_url.to_lowercase() == "refact" {
        vec!["https://inference.smallcloud.ai/coding_assistant_caps.json".to_string()]
    } else {
        let base_url = Url::parse(&cmdline.address_url.clone()).map_err(|_| "failed to parse address url (1)".to_string())?;
        let joined_url = base_url.join(&CAPS_FILENAME).map_err(|_| "failed to parse address url (2)".to_string())?;
        let joined_url_fallback = base_url.join(&CAPS_FILENAME_FALLBACK).map_err(|_| "failed to parse address url (2)".to_string())?;
        caps_urls.push(joined_url.to_string());
        caps_urls.push(joined_url_fallback.to_string());
    }
    
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

    resolve_relative_urls(&mut server_provider, &caps_url)?;
    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;
    
    let (mut providers, error_log) = read_providers_d(vec![server_provider], &config_dir).await;
    for e in error_log {
        tracing::error!("{e}");
    }
    for provider in &mut providers {
        add_running_models(provider);
        populate_model_records(provider);
        apply_models_dict_patch(provider);
        add_name_and_id_to_model_records(provider);
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