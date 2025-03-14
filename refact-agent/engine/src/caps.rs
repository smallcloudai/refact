use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
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
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::known_models::KNOWN_MODELS;


const CAPS_FILENAME: &str = "refact-caps";
const CAPS_FILENAME_FALLBACK: &str = "coding_assistant_caps.json";


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    #[serde(default)]
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_scratchpads: HashMap<String, Value>,
    #[serde(default)]
    pub default_scratchpad: String,
    #[serde(default)]
    pub similar_models: Vec<String>,
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
    #[serde(default)]
    pub tokenizer: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModelType {
    CodeCompletion,
    MultilineCodeCompletion,
    Chat,
    Embedding,
}

#[derive(Debug, Deserialize)]
pub struct ModelsOnly {
    pub code_completion_models: IndexMap<String, ModelRecord>,
    pub code_chat_models: IndexMap<String, ModelRecord>,
    pub tokenizer_rewrite_path: HashMap<String, String>,
}

fn default_tokenizer_path_template() -> String {
    String::from("https://huggingface.co/$MODEL/resolve/main/tokenizer.json")
}

fn default_telemetry_basic_dest() -> String {
    String::from("https://www.smallcloud.ai/v1/telemetry-basic")
}

fn default_telemetry_basic_retrieve_my_own() -> String {
    String::from("https://www.smallcloud.ai/v1/telemetry-retrieve-my-own-stats")
}

fn default_endpoint_style() -> String {
    String::from("openai")
}

fn default_code_completion_n_ctx() -> usize {
    2048
}

fn default_endpoint_embeddings_style() -> String {
    String::from("openai")
}

fn default_support_metadata() -> bool { false }

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantCaps {
    #[serde(default)]
    pub providers: IndexMap<String, CapsProvider>,

    #[serde(default = "default_tokenizer_path_template")]
    pub tokenizer_path_template: String,
    #[serde(default)]
    pub tokenizer_rewrite_path: HashMap<String, String>, // TO-DO, remove this
    
    #[serde(default = "default_telemetry_basic_dest")]
    pub telemetry_basic_dest: String,
    #[serde(default = "default_telemetry_basic_retrieve_my_own")]
    pub telemetry_basic_retrieve_my_own: String,

    #[serde(default, alias = "completion_model")]
    pub code_completion_default_model: String,
    #[serde(default, alias = "completion_provider")]
    pub code_completion_default_provider: String,

    #[serde(default, alias = "multiline_completion_model")]
    pub multiline_code_completion_default_model: String,
    #[serde(default, alias = "multiline_completion_provider")]
    pub multiline_code_completion_default_provider: String,

    #[serde(default, alias = "chat_model")]
    pub code_chat_default_model: String,
    #[serde(default, alias = "chat_provider")]
    pub code_chat_default_provider: String,

    #[serde(default, alias = "default_embeddings_model")]
    pub embedding_model: String,
    #[serde(default, alias = "default_embeddings_provider")]
    pub embedding_provider: String,
    
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes
    #[serde(default)]
    pub code_chat_default_system_prompt: String,

    #[serde(default)]
    pub customization: String,  // on self-hosting server, allows to customize yaml_configs & friends for all engineers
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CapsProvider {
    #[serde(alias = "cloud_name", default)]
    pub name: String,

    #[serde(default = "default_endpoint_style")]
    pub endpoint_style: String,
    #[serde(default)]
    pub chat_endpoint_style: String,
    #[serde(default = "default_endpoint_style")]
    pub completion_endpoint_style: String,
    #[serde(default)]
    pub endpoint_chat_passthrough: String,
    #[serde(default = "default_endpoint_embeddings_style")]
    #[serde(alias = "embedding_endpoint_style")]
    pub endpoint_embeddings_style: String,

    #[serde(default)]
    pub endpoint_template: String,
    #[serde(default)]
    pub completion_endpoint: String,
    #[serde(default)]
    pub chat_endpoint: String,
    #[serde(default)]
    #[serde(alias = "embedding_endpoint")]
    pub endpoint_embeddings_template: String,

    // default api key is in the command line
    #[serde(default)]
    pub completion_apikey: String,
    #[serde(default)]
    pub chat_apikey: String,
    #[serde(default)]
    pub embedding_apikey: String,

    #[serde(default)]
    #[serde(alias = "size_embeddings")]
    pub embedding_size: i32,
    #[serde(default)]
    pub embedding_batch: usize,

    #[serde(default)]
    pub embedding_n_ctx: usize,
    #[serde(default = "default_code_completion_n_ctx")]
    #[serde(alias = "completion_n_ctx")]
    pub code_completion_n_ctx: usize,

    #[serde(default = "default_support_metadata")]
    pub support_metadata: bool,

    #[serde(default)]
    pub code_completion_models: IndexMap<String, ModelRecord>,
    #[serde(default)]
    pub code_chat_models: IndexMap<String, ModelRecord>,
    #[serde(default)]
    pub models_dict_patch: HashMap<String, ModelRecord>,

    #[serde(default)]
    pub running_models: Vec<String>,  // check there if a model is available or not, not in other places
}

async fn read_providers_d(config_dir: &Path) -> (IndexMap<String, CapsProvider>, Vec<YamlError>) {
    let providers_dir = config_dir.join("providers.d");
    let mut providers = IndexMap::new();
    let mut error_log = Vec::new();

    let mut entries = match tokio::fs::read_dir(&providers_dir).await {
        Ok(entries) => entries,
        Err(e) => return {
            tracing::warn!("Failed to read providers directory: {}", e);
            (providers, error_log)
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

        if provider_name == "main" {
            error_log.push(YamlError {
                path: path.to_string_lossy().to_string(),
                error_line: 0,
                error_msg: "Provider name 'main' is reserved, skipping file".to_string(),
            });
            continue;
        }

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

        let provider: CapsProvider = match serde_yaml::from_str(&content) {
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

        providers.insert(provider_name, provider);
    }

    (providers, error_log)
}

fn parse_from_yaml_or_json<'de, T: Deserialize<'de>>(buffer: &'de str) -> Result<T, String> {
    match serde_json::from_str::<T>(&buffer) {
        Ok(v) => Ok(v),
        Err(json_err) => match serde_yaml::from_str::<T>(&buffer) {
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

async fn load_caps_from_buf(
    buffer: &str,
    caps_url: &str,
    config_dir: &Path,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let mut caps: CodeAssistantCaps = parse_from_yaml_or_json(buffer)?;

    let main_provider: CapsProvider = parse_from_yaml_or_json(buffer)?;
    caps.providers.insert(main_provider.name.clone(), main_provider);

    let (providers, error_log) = read_providers_d(config_dir).await;
    for e in error_log {
        tracing::error!("{e}");
    }
    caps.providers.extend(providers.into_iter());

    let known_models: ModelsOnly = serde_json::from_str(&KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;

    let models_to_add = [
        (caps.code_chat_default_model.clone(), caps.code_chat_default_provider.clone()),
        (caps.code_completion_default_model.clone(), caps.code_completion_default_provider.clone()),
        (caps.multiline_code_completion_default_model.clone(), caps.multiline_code_completion_default_provider.clone()),
        (caps.embedding_model.clone(), caps.embedding_provider.clone()),
    ];
    for (model, provider_name) in models_to_add {
        match get_caps_provider_mut(&mut caps, &provider_name) {
            Ok(provider) => {
                if !provider.running_models.contains(&model) {
                    provider.running_models.push(model.clone());
                }
            },
            Err(e) => tracing::error!("{e}"),
        };
    }

    populate_model_dicts(&mut caps, &known_models);
    apply_models_dict_patch(&mut caps);
    for (_name, provider) in &mut caps.providers {
        provider.endpoint_template = relative_to_full_url(caps_url, &provider.endpoint_template)?;
        provider.endpoint_chat_passthrough = relative_to_full_url(caps_url, &provider.endpoint_chat_passthrough)?;
        if provider.endpoint_chat_passthrough.is_empty() {
            provider.endpoint_chat_passthrough = relative_to_full_url(caps_url, &provider.chat_endpoint)?;
        }

        provider.endpoint_embeddings_template = relative_to_full_url(&caps_url, &provider.endpoint_embeddings_template)?;
        if provider.embedding_n_ctx == 0 {
            provider.embedding_n_ctx = 512;
        }
    }

    caps.tokenizer_path_template = relative_to_full_url(&caps_url, &caps.tokenizer_path_template)?;
    caps.telemetry_basic_dest = relative_to_full_url(&caps_url, &caps.telemetry_basic_dest)?;
    caps.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &caps.telemetry_basic_retrieve_my_own)?;
    
    
    // info!("caps {} completion models", caps.code_completion_models.len());
    // info!("caps default completion model: \"{}\"", caps.code_completion_default_model);
    // info!("caps {} chat models", caps.code_chat_models.len());
    // info!("caps default chat model: \"{}\"", caps.code_chat_default_model);
    // info!("running models: {:?}", caps.running_models);
    // info!("code_chat_models models: {:?}", caps.code_chat_models);
    // info!("code completion models: {:?}", caps.code_completion_models);
    Ok(Arc::new(StdRwLock::new(caps)))
}

pub async fn get_api_key(
    api_key_type: ModelType,
    gcx: Arc<ARwLock<GlobalContext>>,
    provider_name: &str,
) -> Result<String, String> {
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0)
        .await.map_err_to_string()?;
    let cmdline_api_key = gcx.read().await.cmdline.api_key.clone();

    let custom_apikey = {
        let caps_locked = caps.read().unwrap();
        let provider = if provider_name.is_empty() {
            caps_locked.providers.first().map(|(_, p)| p)
        } else {
            caps_locked.providers.get(provider_name)
        };
        if let Some(provider) = provider {
            match api_key_type {
                ModelType::Chat => provider.chat_apikey.clone(),
                ModelType::CodeCompletion | ModelType::MultilineCodeCompletion => 
                    provider.completion_apikey.clone(),
                ModelType::Embedding => provider.embedding_apikey.clone(),
            }
        } else {
            String::new()
        }
    };
    
    if custom_apikey.is_empty() {
        Ok(cmdline_api_key)
    } else if custom_apikey.starts_with("$") {
        let env_var_name = &custom_apikey[1..];
        match std::env::var(env_var_name) {
            Ok(env_value) => Ok(env_value),
            Err(e) => {
                error!("tried to read API key from env var {}, but failed: {}\nTry editing ~/.config/refact/bring-your-own-key.yaml", env_var_name, e);
                Ok(cmdline_api_key)
            }
        }
    } else {
        Ok(custom_apikey)
    }
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
    let config_dir = gcx.read().await.config_dir.clone();
    let mut caps_url = cmdline.address_url.clone();
    let buf: String;
    if caps_url.to_lowercase() == "refact" || caps_url.starts_with("http") {
        (buf, caps_url) = load_caps_buf_from_url(cmdline, gcx).await?
    } else {
        (buf, caps_url) = load_caps_buf_from_file(cmdline, gcx).await?
    }
    load_caps_from_buf(&buf, &caps_url, &config_dir).await
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

fn apply_models_dict_patch(caps: &mut CodeAssistantCaps) {
    fn apply_model_record_patch(rec: &mut ModelRecord, rec_patched: &ModelRecord) {
        if rec_patched.n_ctx != 0 {
            rec.n_ctx = rec_patched.n_ctx;
        }
        if rec_patched.supports_tools {
            rec.supports_tools = rec_patched.supports_tools;
        }
        if rec_patched.supports_multimodality {
            rec.supports_multimodality = rec_patched.supports_multimodality;
        }
        if rec_patched.supports_tools {
            rec.supports_tools = rec_patched.supports_tools;
        }
    }

    for provider in caps.providers.values_mut() {
        for (model_name, rec_patched) in provider.models_dict_patch.iter() {
            if let Some(rec) = provider.code_completion_models.get_mut(model_name) {
                apply_model_record_patch(rec, rec_patched);
            }
            if let Some(rec) = provider.code_chat_models.get_mut(model_name) {
                apply_model_record_patch(rec, rec_patched);
            }
        }
    }
}

fn populate_model_dicts(
    caps: &mut CodeAssistantCaps,
    known_models: &ModelsOnly,
) {
    // XXX: only patches running models, patch all?
    for (provider_name, provider) in caps.providers.iter_mut() {
        for caps_model_name in &provider.running_models {
            let caps_model_stripped = strip_model_from_finetune(caps_model_name);

            if !provider.code_completion_models.contains_key(&caps_model_stripped) {
                for (known_model_name, known_model_rec) in &known_models.code_completion_models {
                    if known_model_name == &caps_model_stripped || known_model_rec.similar_models.contains(&caps_model_stripped) {
                        provider.code_completion_models.insert(caps_model_name.clone(), known_model_rec.clone());
                    }
                }
            }

            if!provider.code_chat_models.contains_key(&caps_model_stripped) {
                for (known_model_name, known_model_rec) in &known_models.code_chat_models {
                    if known_model_name == &caps_model_stripped || known_model_rec.similar_models.contains(&caps_model_stripped) {
                        provider.code_chat_models.insert(caps_model_name.clone(), known_model_rec.clone());
                    }
                }
            }
        }

        for model in &provider.running_models {
            if !provider.code_completion_models.contains_key(model) && 
                !provider.code_chat_models.contains_key(model) &&
                !(model == &caps.embedding_model && provider_name == &caps.embedding_provider) {
                tracing::warn!("Indicated as running, unknown model {:?} for provider {}, maybe update this rust binary", model, provider_name);
            }
        }
    }

    for tok_rewrite in known_models.tokenizer_rewrite_path.keys() {
        if !caps.tokenizer_rewrite_path.contains_key(tok_rewrite) {
            caps.tokenizer_rewrite_path.insert(tok_rewrite.to_string(), known_models.tokenizer_rewrite_path[tok_rewrite].clone());
        }
    }
}

pub fn which_model_to_use<'a>(
    model_type: ModelType,
    caps: &'a CodeAssistantCaps,
    user_wants_model: &str,
    user_wants_model_provider: &str,
) -> Result<(String, &'a ModelRecord, &'a CapsProvider), String> {
    let (model_name, provider_name) = if !user_wants_model.is_empty() {
        (user_wants_model, user_wants_model_provider)
    } else {
        match model_type {
            ModelType::CodeCompletion => (
                caps.code_completion_default_model.as_str(),
                caps.code_completion_default_provider.as_str(),
            ),
            ModelType::MultilineCodeCompletion => (
                caps.multiline_code_completion_default_model.as_str(),
                caps.multiline_code_completion_default_provider.as_str(),
            ),
            ModelType::Chat => (
                caps.code_chat_default_model.as_str(),
                caps.code_chat_default_provider.as_str(),
            ),
            ModelType::Embedding => unimplemented!(),
        }
    };
    
    let model_name_stripped = strip_model_from_finetune(model_name);

    let provider = get_caps_provider(&caps, provider_name)?;
    let models_list = match model_type {
        ModelType::CodeCompletion | ModelType::MultilineCodeCompletion => &provider.code_completion_models,
        ModelType::Chat => &provider.code_chat_models,
        ModelType::Embedding => unimplemented!(),
    };

    let model_rec = models_list.get(model_name)
        .or_else(|| models_list.get(&model_name_stripped));

    match model_rec {
        Some(model_rec) => Ok((model_name.to_string(), model_rec, provider)),
        None => Err(format!(
            "Model '{}' not found. Server has the following models for provider {}: {:?}",
            model_name,
            provider_name,
            models_list.keys()
        )),
    }
}

pub fn which_scratchpad_to_use<'a>(
    scratchpads: &'a HashMap<String, serde_json::Value>,
    user_wants_scratchpad: &str,
    default_scratchpad: &str,
) -> Result<(String, &'a serde_json::Value), String> {
    let mut take_this_one = default_scratchpad;
    if user_wants_scratchpad != "" {
        take_this_one = user_wants_scratchpad;
    }
    if default_scratchpad == "" {
        if scratchpads.len() == 1 {
            let key = scratchpads.keys().next().unwrap();
            return Ok((key.clone(), &scratchpads[key]));
        } else {
            return Err(format!(
                "There is no default scratchpad defined, requested scratchpad is empty. The model supports these scratchpads: {:?}",
                scratchpads.keys()
            ));
        }
    }
    if let Some(scratchpad_patch) = scratchpads.get(take_this_one) {
        return Ok((take_this_one.to_string(), scratchpad_patch));
    } else {
        return Err(format!(
            "Scratchpad '{}' not found. The model supports these scratchpads: {:?}",
            take_this_one,
            scratchpads.keys()
        ));
    }
}

pub fn get_caps_provider<'a>(caps: &'a CodeAssistantCaps, provider_name: &str) -> Result<&'a CapsProvider, String> {
    if provider_name.is_empty() {
        caps.providers.first().map(|(_, p)| p)
            .ok_or("No providers defined in caps".to_string())
    } else {
        caps.providers.get(provider_name)
            .ok_or(format!("No provider named `{}` in caps", provider_name))
    }
}

pub fn get_caps_provider_mut<'a>(caps: &'a mut CodeAssistantCaps, provider_name: &str) -> Result<&'a mut CapsProvider, String> {
    if provider_name.is_empty() {
        caps.providers.first_mut().map(|(_, p)| p)
            .ok_or("No providers defined in caps".to_string())
    } else {
        caps.providers.get_mut(provider_name)
            .ok_or(format!("No provider named `{}` in caps", provider_name))
    }
}

pub async fn get_model_record(
    gcx: Arc<ARwLock<GlobalContext>>,
    model: &str,
    provider_name: &str,
) -> Result<ModelRecord, String> {
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(
        gcx.clone(), 0,
    ).await.map_err(|e| {
        warn!("no caps: {:?}", e);
        format!("failed to load caps: {}", e)
    })?;

    let caps_locked = caps.read().unwrap();
    let provider = get_caps_provider(&caps_locked, provider_name)?;
    match provider.code_chat_models.get(model) {
        Some(res) => Ok(res.clone()),
        None => Err(format!("no model record for model `{}`", model))
    }
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