use std::path::PathBuf;
use std::collections::HashMap;
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

use crate::custom_error::ScratchError;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::known_models::KNOWN_MODELS;


const CAPS_FILENAME: &str = "refact-caps";
const CAPS_FILENAME_FALLBACK: &str = "coding_assistant_caps.json";


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    #[serde(default)]
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_scratchpads: HashMap<String, serde_json::Value>,
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
}

#[derive(Debug, Deserialize)]
pub struct ModelsOnly {
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_chat_models: HashMap<String, ModelRecord>,
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
    pub cloud_name: String,

    #[serde(default = "default_endpoint_style")]
    pub endpoint_style: String,
    #[serde(default)]
    pub chat_endpoint_style: String,
    #[serde(default = "default_endpoint_style")]
    pub completion_endpoint_style: String,

    #[serde(default)]
    pub endpoint_template: String,
    #[serde(default)]
    pub completion_endpoint: String,
    #[serde(default)]
    pub chat_endpoint: String,

    // default api key is in the command line
    #[serde(default)]
    pub completion_apikey: String,
    #[serde(default)]
    pub chat_apikey: String,
    #[serde(default)]
    pub embedding_apikey: String,

    #[serde(default)]
    pub endpoint_chat_passthrough: String,
    #[serde(default = "default_tokenizer_path_template")]
    pub tokenizer_path_template: String,
    #[serde(default)]
    pub tokenizer_rewrite_path: HashMap<String, String>,
    #[serde(default = "default_telemetry_basic_dest")]
    pub telemetry_basic_dest: String,
    #[serde(default = "default_telemetry_basic_retrieve_my_own")]
    pub telemetry_basic_retrieve_my_own: String,
    #[serde(default)]
    pub code_completion_models: HashMap<String, ModelRecord>,
    #[serde(default)]
    #[serde(alias = "completion_model")]
    pub code_completion_default_model: String,
    #[serde(default)]
    #[serde(alias = "multiline_completion_model")]
    pub multiline_code_completion_default_model: String,
    #[serde(default = "default_code_completion_n_ctx")]
    #[serde(alias = "completion_n_ctx")]
    pub code_completion_n_ctx: usize,
    #[serde(default)]
    pub code_chat_models: HashMap<String, ModelRecord>,
    #[serde(default)]
    #[serde(alias = "chat_model")]
    pub code_chat_default_model: String,
    #[serde(default)]
    pub models_dict_patch: HashMap<String, ModelRecord>,
    #[serde(default)]
    #[serde(alias = "default_embeddings_model")]
    pub embedding_model: String,
    #[serde(default)]
    #[serde(alias = "embedding_endpoint")]
    pub endpoint_embeddings_template: String,
    #[serde(default = "default_endpoint_embeddings_style")]
    #[serde(alias = "embedding_endpoint_style")]
    pub endpoint_embeddings_style: String,
    #[serde(default)]
    #[serde(alias = "size_embeddings")]
    pub embedding_size: i32,
    #[serde(default)]
    pub embedding_batch: usize,
    #[serde(default)]
    pub embedding_n_ctx: usize,
    #[serde(default)]
    pub running_models: Vec<String>,  // check there if a model is available or not, not in other places
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes
    // #[serde(default)]
    // pub code_chat_default_system_prompt: String,

    #[serde(default)]
    pub customization: String,  // on self-hosting server, allows to customize yaml_configs & friends for all engineers

    #[serde(default = "default_support_metadata")]
    pub support_metadata: bool,
}

fn load_caps_from_buf(
    buffer: &String,
    caps_url: &String,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let mut r1_mb_error_text = "".to_string();

    let r1_mb: Option<CodeAssistantCaps> = match serde_json::from_str(&buffer) {
        Ok(v) => v,
        Err(e) => {
            // incorrect json
            if buffer.trim_start().starts_with(&['{', '[']) {
                r1_mb_error_text = format!("{}", e);
                None
            } else {
                match serde_yaml::from_str(&buffer) {
                    Ok(v) => v,
                    Err(e) => {
                        r1_mb_error_text = format!("{}", e);
                        None
                    }
                }
            }
        }
    };
    let mut r1 = r1_mb.ok_or(format!("failed to parse caps: {}", r1_mb_error_text))?;

    let r0: ModelsOnly = serde_json::from_str(&KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;

    if !r1.code_chat_default_model.is_empty() && !r1.running_models.contains(&r1.code_chat_default_model) {
        r1.running_models.push(r1.code_chat_default_model.clone());
    }
    if !r1.code_completion_default_model.is_empty() && !r1.running_models.contains(&r1.code_completion_default_model) {
        r1.running_models.push(r1.code_completion_default_model.clone());
    }
    if !r1.multiline_code_completion_default_model.is_empty() && !r1.running_models.contains(&r1.multiline_code_completion_default_model) {
        r1.running_models.push(r1.multiline_code_completion_default_model.clone());
    }
    if !r1.embedding_model.is_empty() && !r1.running_models.contains(&r1.embedding_model) {
        r1.running_models.push(r1.embedding_model.clone());
    }

    _inherit_r1_from_r0(&mut r1, &r0);
    apply_models_dict_patch(&mut r1);
    r1.endpoint_template = relative_to_full_url(&caps_url, &r1.endpoint_template)?;
    r1.endpoint_chat_passthrough = relative_to_full_url(&caps_url, &r1.endpoint_chat_passthrough)?;
    if r1.endpoint_chat_passthrough.is_empty() {
        r1.endpoint_chat_passthrough = relative_to_full_url(&caps_url, &r1.chat_endpoint)?;
    }
    r1.telemetry_basic_dest = relative_to_full_url(&caps_url, &r1.telemetry_basic_dest)?;
    r1.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &r1.telemetry_basic_retrieve_my_own)?;
    r1.endpoint_embeddings_template = relative_to_full_url(&caps_url, &r1.endpoint_embeddings_template)?;
    r1.tokenizer_path_template = relative_to_full_url(&caps_url, &r1.tokenizer_path_template)?;
    if r1.embedding_n_ctx == 0 {
        r1.embedding_n_ctx = 512;
    }

    // info!("caps {} completion models", r1.code_completion_models.len());
    // info!("caps default completion model: \"{}\"", r1.code_completion_default_model);
    // info!("caps {} chat models", r1.code_chat_models.len());
    // info!("caps default chat model: \"{}\"", r1.code_chat_default_model);
    // info!("running models: {:?}", r1.running_models);
    // info!("code_chat_models models: {:?}", r1.code_chat_models);
    // info!("code completion models: {:?}", r1.code_completion_models);
    Ok(Arc::new(StdRwLock::new(r1)))
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
    let mut caps_url = cmdline.address_url.clone();
    let buf: String;
    if caps_url.to_lowercase() == "refact" || caps_url.starts_with("http") {
        (buf, caps_url) = load_caps_buf_from_url(cmdline, gcx).await?
    } else {
        (buf, caps_url) = load_caps_buf_from_file(cmdline, gcx).await?
    }
    load_caps_from_buf(&buf, &caps_url)
}

pub fn strip_model_from_finetune(model: &String) -> String {
    model.split(":").next().unwrap().to_string()
}

fn relative_to_full_url(
    caps_url: &String,
    maybe_relative_url: &str,
) -> Result<String, String> {
    if maybe_relative_url.starts_with("http") {
        Ok(maybe_relative_url.to_string())
    } else if maybe_relative_url.is_empty() {
        Ok("".to_string())
    } else {
        let base_url = Url::parse(caps_url.as_str()).map_err(|_| "failed to parse address url (3)".to_string())?;
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

    for (model, rec_patched) in caps.models_dict_patch.iter() {
        if let Some(rec) = caps.code_completion_models.get_mut(model) {
            apply_model_record_patch(rec, rec_patched);
        }
        if let Some(rec) = caps.code_chat_models.get_mut(model) {
            apply_model_record_patch(rec, rec_patched);
        }
    }
}

fn _inherit_r1_from_r0(
    r1: &mut CodeAssistantCaps,
    r0: &ModelsOnly,
) {
    // XXX: only patches running models, patch all?
    for k in r1.running_models.iter() {
        let k_stripped = strip_model_from_finetune(k);

        for (rec_name, rec) in r0.code_completion_models.iter() {
            if rec_name == &k_stripped || rec.similar_models.contains(&k_stripped) {
                r1.code_completion_models.insert(k.to_string(), rec.clone());
            }
        }

        for (rec_name, rec) in r0.code_chat_models.iter() {
            if rec_name == &k_stripped || rec.similar_models.contains(&k_stripped) {
                r1.code_chat_models.insert(k.to_string(), rec.clone());
            }
        }
    }

    for k in r1.running_models.iter() {
        if !r1.code_completion_models.contains_key(k) && !r1.code_chat_models.contains_key(k) && *k != r1.embedding_model {
            warn!("indicated as running, unknown model {:?}, maybe update this rust binary", k);
        }
    }

    for k in r0.tokenizer_rewrite_path.keys() {
        if !r1.tokenizer_rewrite_path.contains_key(k) {
            r1.tokenizer_rewrite_path.insert(k.to_string(), r0.tokenizer_rewrite_path[k].clone());
        }
    }
}

pub fn which_model_to_use<'a>(
    models: &'a HashMap<String, ModelRecord>,
    user_wants_model: &str,
    default_model: &str,
) -> Result<(String, &'a ModelRecord), String> {
    let mut take_this_one = default_model;
    if user_wants_model != "" {
        take_this_one = user_wants_model;
    }
    let no_finetune = strip_model_from_finetune(&take_this_one.to_string());
    if let Some(model_rec) = models.get(&take_this_one.to_string()) {
        return Ok((take_this_one.to_string(), model_rec));
    } else if let Some(model_rec) = models.get(&no_finetune) {
        return Ok((take_this_one.to_string(), model_rec));
    } else {
        return Err(format!(
            "Model '{}' not found. Server has these models: {:?}",
            take_this_one,
            models.keys()
        ));
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

pub async fn get_model_record(
    gcx: Arc<ARwLock<GlobalContext>>,
    model: &str,
) -> Result<ModelRecord, String> {
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(
        gcx.clone(), 0,
    ).await.map_err(|e| {
        warn!("no caps: {:?}", e);
        format!("failed to load caps: {}", e)
    })?;

    let caps_lock = caps.read().unwrap();
    match caps_lock.code_chat_models.get(model) {
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
