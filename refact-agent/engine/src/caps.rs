use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
use serde::Deserializer;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock as ARwLock;
use url::Url;
use tracing::{error, info, warn};

use crate::custom_error::MapErrToString;
use crate::custom_error::YamlError;
use crate::global_context::GlobalContext;
use crate::known_models::KNOWN_MODELS;


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

    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub endpoint_style: String,
    #[serde(default)]
    pub api_key: String,

    #[serde(default)]
    pub support_metadata: bool,
    #[serde(default)]
    pub similar_models: Vec<String>,
    #[serde(default)]
    pub tokenizer: String,
}

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct ScratchpadSupport {
    #[serde(default, rename = "supports_scratchpads")]
    pub supported: HashMap<String, Value>,
    #[serde(default, rename = "default_scratchpad")]
    pub default: String,
}

impl ScratchpadSupport {
    pub fn resolve<'a>(&'a self, user_wants: &str) -> Result<(String, &'a Value), String> {
        let name = if !user_wants.is_empty() { user_wants } else { &self.default };
        
        // Special case: no default but exactly one scratchpad exists
        if name.is_empty() && self.supported.len() == 1 {
            let (name, scratchpad) = self.supported.iter().next().unwrap();
            return Ok((name.to_string(), scratchpad));
        }
        
        self.supported.get(name)
            .map(|value| (name.to_string(), value))
            .ok_or_else(|| format!(
                "Scratchpad '{}' not found. Available scratchpads: {:?}",
                name, self.supported.keys()
            ))
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct ChatModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,
    #[serde(flatten)]
    pub scratchpads: ScratchpadSupport,

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

#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct CompletionModelRecord {
    #[serde(flatten)]
    pub base: BaseModelRecord,
    #[serde(flatten)]
    pub scratchpads: ScratchpadSupport,
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
    pub code_completion_models: IndexMap<String, CompletionModelRecord>,
    #[serde(skip_deserializing)]
    pub code_chat_models: IndexMap<String, ChatModelRecord>,
    #[serde(skip_deserializing)]
    pub embedding_model: EmbeddingModelRecord,

    #[serde(flatten)]
    pub default_models: DefaultModels,
    
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes
    #[serde(default)]
    pub code_chat_default_system_prompt: String, // Should we get rid of this?

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
pub struct CapsProvider {
    #[serde(alias = "cloud_name", default)]
    pub name: String,

    #[serde(default = "default_endpoint_style")]
    pub endpoint_style: String,

    #[serde(default)]
    pub endpoint_template: String, // We can get rid easily of this
    #[serde(default)]
    pub completion_endpoint: String,
    #[serde(default, alias = "endpoint_chat_passthrough")]
    pub chat_endpoint: String,
    #[serde(default, alias = "endpoint_embeddings_template")]
    pub embedding_endpoint: String,

    // default api key is in the command line
    #[serde(default)]
    pub api_key: String,

    #[serde(default = "default_code_completion_n_ctx")]
    pub code_completion_n_ctx: usize,

    #[serde(default)]
    pub support_metadata: bool,

    #[serde(default)]
    pub code_completion_models: IndexMap<String, CompletionModelRecord>,
    #[serde(default)]
    pub code_chat_models: IndexMap<String, ChatModelRecord>,
    #[serde(default, alias = "default_embeddings_model", deserialize_with = "deserialize_embedding_model")]
    pub embedding_model: EmbeddingModelRecord,

    #[serde(default)]
    pub models_dict_patch: IndexMap<String, Value>, // Used to patch some params from cloud, like n_ctx for pro/free users

    #[serde(flatten)]
    pub default_models: DefaultModels,

    #[serde(default)]
    pub running_models: Vec<String>,
}

fn default_endpoint_style() -> String { "openai".to_string() }

fn default_code_completion_n_ctx() -> usize { 2048 }

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DefaultModels {
    #[serde(default, alias = "code_completion_default_model")]
    pub completion_model: String,
    #[serde(default, alias = "multiline_code_completion_default_model")]
    pub multiline_completion_model: String,
    #[serde(default, alias = "code_chat_default_model")]
    pub chat_model: String,
}

fn deserialize_embedding_model<'de, D: Deserializer<'de>>(
    deserializer: D
) -> Result<EmbeddingModelRecord, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Input { String(String), Full(EmbeddingModelRecord) }
    
    match Input::deserialize(deserializer)? {
        Input::String(name) => {
            Ok(EmbeddingModelRecord { 
            base: BaseModelRecord { name, ..Default::default() }, ..Default::default() 
        })},
        Input::Full(record) => Ok(record),
    }
}

async fn read_providers_d(
    prev_providers: Vec<CapsProvider>, 
    config_dir: &Path
) -> (Vec<CapsProvider>, Vec<YamlError>) {
    let providers_dir = config_dir.join("providers.d");
    let mut providers = prev_providers;
    let mut error_log = Vec::new();

    let mut entries = match tokio::fs::read_dir(&providers_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to read providers directory: {}", e);
            return (providers, error_log);
        }
    };

    while let Some(entry_result) = entries.next_entry().await.transpose() {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                error_log.push(YamlError {
                    path: providers_dir.to_string_lossy().to_string(),
                    error_line: 0,
                    error_msg: e.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        
        if !path.is_file() || 
           !(path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml")) {
            continue;
        }

        let provider_name = match path.file_stem() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(e) => {
                error_log.push(YamlError {
                    path: path.to_string_lossy().to_string(),
                    error_line: 0,
                    error_msg: format!("Failed to read file: {}", e),
                });
                continue;
            }
        };

        let mut provider: CapsProvider = match serde_yaml::from_str(&content) {
            Ok(provider) => provider,
            Err(e) => {
                error_log.push(YamlError {
                    path: path.to_string_lossy().to_string(),
                    error_line: e.location().map_or(0, |loc| loc.line()),
                    error_msg: format!("Failed to parse YAML: {}", e),
                });
                continue;
            }
        };
        provider.name = provider_name;

        let mut models_to_add = vec![
            &provider.default_models.chat_model,
            &provider.default_models.completion_model,
            &provider.default_models.multiline_completion_model,
        ];
        models_to_add.extend(provider.code_chat_models.keys());
        models_to_add.extend(provider.code_completion_models.keys());

        for model in models_to_add {
            if !model.is_empty() && !provider.running_models.contains(model) {
                provider.running_models.push(model.clone());
            }
        }

        providers.push(provider);
    }

    (providers, error_log)
}

fn parse_from_yaml_or_json(buffer: &str) -> Result<serde_json::Value, String> {
    match serde_json::from_str::<serde_json::Value>(buffer) {
        Ok(v) => Ok(v),
        Err(json_err) => match serde_yaml::from_str::<serde_json::Value>(buffer) {
            Ok(v) => Ok(v),
            Err(yaml_err) => {
                if buffer.trim_start().starts_with(&['{', '[']) {
                    return Err(format!("failed to parse caps: {}", json_err));
                } else {
                    return Err(format!("failed to parse caps: {}", yaml_err));
                }
            }
        },
    }
}

fn add_models_to_caps(caps: &mut CodeAssistantCaps, providers: Vec<CapsProvider>) {
    fn add_provider_details_to_model(base_model_rec: &mut BaseModelRecord, provider: &CapsProvider, model_name: &str, custom_endpoint: &str) {
        base_model_rec.name = model_name.to_string();
        base_model_rec.id = format!("{}/{}", provider.name, model_name);
        
        base_model_rec.api_key = provider.api_key.clone();
            
        let endpoint = if !custom_endpoint.is_empty() {
            custom_endpoint.to_string()
        } else {
            provider.endpoint_template.clone()
        };
        base_model_rec.endpoint = endpoint.replace("$MODEL", model_name);
        base_model_rec.support_metadata = provider.support_metadata;
        base_model_rec.endpoint_style = provider.endpoint_style.clone();
    }
    
    for mut provider in providers {

        let completion_models = std::mem::take(&mut provider.code_completion_models);
        for (model_name, mut model_rec) in completion_models {
            add_provider_details_to_model(
                &mut model_rec.base, &provider, &model_name, &provider.completion_endpoint
            );

            if provider.code_completion_n_ctx > 0 && provider.code_completion_n_ctx < model_rec.base.n_ctx {
                // model is capable of more, but we may limit it from server or provider, e.x. for latency
                model_rec.base.n_ctx = provider.code_completion_n_ctx; 
            }
            
            caps.code_completion_models.insert(model_rec.base.id.clone(), model_rec);
        }

        let chat_models = std::mem::take(&mut provider.code_chat_models);
        for (model_name, mut model_rec) in chat_models {
            add_provider_details_to_model(
                &mut model_rec.base, &provider, &model_name, &provider.chat_endpoint
            );

            caps.code_chat_models.insert(model_rec.base.id.clone(), model_rec);
        }

        if provider.embedding_model.is_configured() {
            let mut embedding_model = std::mem::take(&mut provider.embedding_model);
            let model_name = embedding_model.base.name.clone();
            add_provider_details_to_model(
                &mut embedding_model.base, &provider, &model_name, &provider.embedding_endpoint
            );

            embedding_model.embedding_batch = match embedding_model.embedding_batch {
                0 => 64,
                b if b > 256 => {
                    tracing::warn!("embedding_batch can't be higher than 256");
                    64
                },
                b => b,
            };
            caps.embedding_model = embedding_model;
        }
    }
}

async fn load_caps_from_buf(
    buffer: &str,
    caps_url: &str,
    config_dir: &Path,
    cmdline_api_key: &str,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let buffer_value = parse_from_yaml_or_json(buffer)?;
    let mut caps = serde_json::from_value::<CodeAssistantCaps>(buffer_value.clone())
        .map_err_with_prefix("Failed to parse caps:")?;
    let first_provider = serde_json::from_value::<CapsProvider>(buffer_value)
        .map_err_with_prefix("Failed to parse caps provider:")?;
    
    let (mut providers, error_log) = read_providers_d(vec![first_provider], config_dir).await;
    for e in error_log {
        tracing::error!("{e}");
    }

    populate_with_known_models(&mut providers)?;
    apply_models_dict_patch(&mut providers);

    for provider in &mut providers {
        provider.endpoint_template = relative_to_full_url(caps_url, &provider.endpoint_template)?;
        provider.chat_endpoint = relative_to_full_url(caps_url, &provider.chat_endpoint)?;
        provider.completion_endpoint = relative_to_full_url(caps_url, &provider.completion_endpoint)?;
        provider.embedding_endpoint = relative_to_full_url(caps_url, &provider.embedding_endpoint)?;

        provider.api_key = match &provider.api_key {
            k if k.is_empty() => cmdline_api_key.to_string(),
            k if k.starts_with("$") => {
                match std::env::var(&k[1..]) {
                    Ok(env_val) => env_val,
                    Err(e) => {
                        tracing::error!(
                            "tried to read API key from env var {} for provider {}, but failed: {}", 
                            k, provider.name, e
                        );
                        cmdline_api_key.to_string()
                    }
                }
            }
            k => k.to_string(),
        };
    }
    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;
    
    add_models_to_caps(&mut caps, providers);
    // info!("caps {} completion models", caps.code_completion_models.len());
    // info!("caps default completion model: \"{}\"", caps.code_completion_default_model);
    // info!("caps {} chat models", caps.code_chat_models.len());
    // info!("caps default chat model: \"{}\"", caps.code_chat_default_model);
    // info!("running models: {:?}", caps.running_models);
    // info!("code_chat_models models: {:?}", caps.code_chat_models);
    // info!("code completion models: {:?}", caps.code_completion_models);
    Ok(Arc::new(StdRwLock::new(caps)))
}

async fn load_caps_buf_from_file(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(String, String), String> {
    let mut caps_url = cmdline.address_url.clone();
    if caps_url.is_empty() {
        let config_dir = {
            let gcx_locked = gcx.read().await;
            gcx_locked.config_dir.clone()
        };
        let caps_path = PathBuf::from(config_dir).join("bring-your-own-key.yaml");
        caps_url = caps_path.to_string_lossy().into_owned();
        // info!("will use {} as the caps file", caps_url);
    }
    let mut buffer = String::new();
    let mut file = File::open(caps_url.clone()).map_err(|_| format!("failed to open file '{}'", caps_url))?;
    file.read_to_string(&mut buffer).map_err(|_| format!("failed to read file '{}'", caps_url))?;
    Ok((buffer, caps_url))
}

async fn load_caps_buf_from_url(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(String, String), String> {
    let mut buffer = String::new();
    let mut caps_urls: Vec<String> = Vec::new();
    if cmdline.address_url.to_lowercase() == "refact" {
        caps_urls.push("https://inference.smallcloud.ai/coding_assistant_caps.json".to_string());
    } else {
        let base_url = Url::parse(&cmdline.address_url.clone()).map_err(|_| "failed to parse address url (1)".to_string())?;
        let joined_url = base_url.join(&CAPS_FILENAME).map_err(|_| "failed to parse address url (2)".to_string())?;
        let joined_url_fallback = base_url.join(&CAPS_FILENAME_FALLBACK).map_err(|_| "failed to parse address url (2)".to_string())?;
        caps_urls.push(joined_url.to_string());
        caps_urls.push(joined_url_fallback.to_string());
    }

    let http_client = gcx.read().await.http_client.clone();
    let api_key = cmdline.api_key.clone();
    let mut headers = reqwest::header::HeaderMap::new();
    if !api_key.is_empty() {
        headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(format!("Bearer {}", api_key).as_str()).unwrap());
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_str(format!("refact-lsp {}", crate::version::build_info::PKG_VERSION).as_str()).unwrap());
    }

    let mut status: u16 = 0;
    for url in caps_urls.iter() {
        info!("fetching caps from {}", url);
        let response = http_client.get(url).headers(headers.clone()).send().await.map_err(|e| format!("{}", e))?;
        status = response.status().as_u16();
        buffer = match response.text().await {
            Ok(v) => v,
            Err(_) => continue
        };

        if status == 200 {
            break;
        }

        warn!("status={}; server responded with:\n{}", status, buffer);
    }
    if status != 200 {
        let response_json: serde_json::Result<Value> = serde_json::from_str(&buffer);
        return if let Ok(response_json) = response_json {
            if let Some(detail) = response_json.get("detail") {
                Err(detail.as_str().unwrap().to_string())
            } else {
                Err(format!("cannot fetch caps, status={}", status))
            }
        } else {
            Err(format!("cannot fetch caps, status={}", status))
        };
    }

    let caps_url: String = match caps_urls.get(0) {
        Some(u) => u.clone(),
        None => return Err("caps_url is none".to_string())
    };

    Ok((buffer, caps_url))
}

pub async fn load_caps(
    cmdline: crate::global_context::CommandLine,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let (config_dir, cmdline_api_key) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.config_dir.clone(), gcx_locked.cmdline.api_key.clone())
    };
    let mut caps_url = cmdline.address_url.clone();
    let buf: String;
    if caps_url.to_lowercase() == "refact" || caps_url.starts_with("http") {
        (buf, caps_url) = load_caps_buf_from_url(cmdline, gcx).await?
    } else {
        (buf, caps_url) = load_caps_buf_from_file(cmdline, gcx).await?
    }
    load_caps_from_buf(&buf, &caps_url, &config_dir, &cmdline_api_key).await
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

fn apply_models_dict_patch(providers: &mut Vec<CapsProvider>) {
    for provider in providers {
        for (model_name, rec_patched) in provider.models_dict_patch.iter() {
            if let Some(completion_rec) = provider.code_completion_models.get_mut(model_name) {
                if let Some(n_ctx) = rec_patched.get("n_ctx").and_then(|v| v.as_u64()) {
                    completion_rec.base.n_ctx = n_ctx as usize;
                }
            }
            
            if let Some(chat_rec) = provider.code_chat_models.get_mut(model_name) {
                if let Some(n_ctx) = rec_patched.get("n_ctx").and_then(|v| v.as_u64()) {
                    chat_rec.base.n_ctx = n_ctx as usize;
                }
                
                if let Some(supports_tools) = rec_patched.get("supports_tools").and_then(|v| v.as_bool()) {
                    chat_rec.supports_tools = supports_tools;
                }
                if let Some(supports_multimodality) = rec_patched.get("supports_multimodality").and_then(|v| v.as_bool()) {
                    chat_rec.supports_multimodality = supports_multimodality;
                }
            }
        }
    }
}

fn populate_with_known_models(providers: &mut Vec<CapsProvider>) -> Result<(), String> {
    #[derive(Deserialize)]
    struct KnownModels {
        code_completion_models: IndexMap<String, CompletionModelRecord>,
        code_chat_models: IndexMap<String, ChatModelRecord>,
        embedding_models: IndexMap<String, EmbeddingModelRecord>,
    }
    let known_models: KnownModels = serde_json::from_str(KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;

    for provider in providers {
        for model_name in &provider.running_models {
            let model_stripped = strip_model_from_finetune(model_name);

            if !provider.code_completion_models.contains_key(&model_stripped) {
                for (known_model_name, known_model_rec) in &known_models.code_completion_models {
                    if known_model_name == &model_stripped || known_model_rec.base.similar_models.contains(&model_stripped) {
                        provider.code_completion_models.insert(model_name.clone(), known_model_rec.clone());
                    }
                }
            }

            if !provider.code_chat_models.contains_key(&model_stripped) {
                for (known_model_name, known_model_rec) in &known_models.code_chat_models {
                    if known_model_name == &model_stripped || known_model_rec.base.similar_models.contains(&model_stripped) {
                        provider.code_chat_models.insert(model_name.clone(), known_model_rec.clone());
                    }
                }
            }
        }

        for model in &provider.running_models {
            if !provider.code_completion_models.contains_key(model) && 
                !provider.code_chat_models.contains_key(model) &&
                !(model == &provider.embedding_model.base.name) {
                tracing::warn!("Indicated as running, unknown model {:?} for provider {}, maybe update this rust binary", model, provider.name);
            }
        }

        if !provider.embedding_model.is_configured() && !provider.embedding_model.base.name.is_empty() {
            let model_name = provider.embedding_model.base.name.clone();
            let model_stripped = strip_model_from_finetune(&model_name);
            if let Some(known_model_rec) = known_models.embedding_models.get(&model_stripped) {
                provider.embedding_model = known_model_rec.clone();
                provider.embedding_model.base.name = model_name;
            } else {
                tracing::warn!("Unkown embedding model '{}', maybe configure it or update this binary", model_stripped);
            }
        }
    }

    Ok(())
}

pub fn resolve_model<T: Clone>(
    models: &IndexMap<String, T>,
    requested_model_id: &str,
    default_model_id: &str,
) -> Result<T, String> {
    let model_id = if !requested_model_id.is_empty() {
        requested_model_id
    } else {
        default_model_id
    };
    models.get(model_id).or_else(
        || models.get(&strip_model_from_finetune(model_id))
    ).cloned().ok_or(format!("Model '{}' not found. Server has the following models: {:?}", model_id, models.keys()))
}

pub fn resolve_chat_model(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    requested_model_id: &str,
) -> Result<ChatModelRecord, String> {
    let caps_locked = caps.read().unwrap();
    resolve_model(
        &caps_locked.code_chat_models, 
        requested_model_id, 
        &caps_locked.default_models.chat_model
    )
}

pub fn resolve_completion_model(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    requested_model_id: &str,
) -> Result<CompletionModelRecord, String> {
    let caps_locked = caps.read().unwrap();
    resolve_model(
        &caps_locked.code_completion_models, 
        requested_model_id, 
        &caps_locked.default_models.completion_model
    )
}

pub const BRING_YOUR_OWN_KEY_SAMPLE: &str = r#"
cloud_name: My own mix of clouds!

chat_endpoint: "https://api.openai.com/v1/chat/completions"
chat_apikey: "$OPENAI_API_KEY"           # Will work if you have it in global environment variables, but better use the real sk-... key
chat_model: gpt-4o-mini

embedding_endpoint: "https://api.openai.com/v1/embeddings"
embedding_apikey: "$OPENAI_API_KEY"
embedding_model: text-embedding-3-small
embedding_size: 1536

# completion_endpoint: "https://api-inference.huggingface.co/models/$MODEL"
# completion_endpoint_style: "hf"
# completion_apikey: "hf_..."    # or use $HF_TOKEN if you have it in global environment variables
# completion_model: bigcode/starcoder2-3b

running_models:   # all models mentioned in *_model are automatically running, but you can add more
  - gpt-4o-mini
  - gpt-4o

# More examples https://github.com/smallcloudai/refact-lsp/tree/dev/bring_your_own_key

# Refact sends basic telemetry (counters and errors), you can send it to a different address (a Refact self-hosting server is especially useful) or set to an empty string for no telemetry.
# telemetry_basic_dest: <your-telemetry-address>             # default: https://www.smallcloud.ai/v1/telemetry-basic
# telemetry_basic_retrieve_my_own: <your-telemetry-address>  # default: https://www.smallcloud.ai/v1/telemetry-retrieve-my-own-stats
"#;