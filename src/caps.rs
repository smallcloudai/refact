use tracing::{info, warn, error};
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock;
use url::Url;
use crate::global_context::GlobalContext;
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
}

#[derive(Debug, Deserialize)]
pub struct ModelsOnly {
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_chat_models: HashMap<String, ModelRecord>,
    pub tokenizer_rewrite_path: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantCaps {
    pub cloud_name: String,
    pub endpoint_style: String,
    pub endpoint_template: String,
    #[serde(default)]
    pub endpoint_chat_passthrough: String,
    pub tokenizer_path_template: String,
    pub tokenizer_rewrite_path: HashMap<String, String>,
    #[serde(default)]
    pub telemetry_basic_dest: String,
    #[serde(default)]
    pub telemetry_basic_retrieve_my_own: String,
    #[serde(default)]
    pub telemetry_corrected_snippets_dest: String,
    #[serde(default)]
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_completion_default_model: String,
    #[serde(default)]
    pub code_completion_n_ctx: usize,
    #[serde(default)]
    pub code_chat_models: HashMap<String, ModelRecord>,
    pub code_chat_default_model: String,
    #[serde(default)]
    pub models_dict_patch: HashMap<String, ModelRecord>,
    #[serde(default)]
    pub default_embeddings_model: String,
    #[serde(default)]
    pub endpoint_embeddings_template: String,
    #[serde(default)]
    pub endpoint_embeddings_style: String,
    #[serde(default)]
    pub size_embeddings: i32,
    #[serde(default)]
    pub embedding_n_ctx: usize,
    pub running_models: Vec<String>,
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes
    #[serde(default)]
    pub code_chat_default_system_prompt: String,
    #[serde(default)]
    pub customization: String,
}

pub async fn load_caps(
    cmdline: crate::global_context::CommandLine,
    global_context: Arc<RwLock<GlobalContext>>,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let mut buffer = String::new();
    let mut is_local_file = false;
    let mut is_remote_address = false;
    let mut caps_urls: Vec<String> = Vec::new();
    if cmdline.address_url == "Refact" {
        is_remote_address = true;
        caps_urls.push("https://inference.smallcloud.ai/coding_assistant_caps.json".to_string());
    } else if cmdline.address_url == "HF" {
        buffer = HF_DEFAULT_CAPS.to_string();
        caps_urls.push("<compiled-in-caps-hf>".to_string());
    } else {
        if cmdline.address_url.starts_with("http") {
            is_remote_address = true;
            let base_url = Url::parse(&cmdline.address_url.clone()).map_err(|_| "failed to parse address url (1)".to_string())?;
            let joined_url = base_url.join(&CAPS_FILENAME).map_err(|_| "failed to parse address url (2)".to_string())?;
            let joined_url_fallback = base_url.join(&CAPS_FILENAME_FALLBACK).map_err(|_| "failed to parse address url (2)".to_string())?;
            caps_urls.push(joined_url.to_string());
            caps_urls.push(joined_url_fallback.to_string());
        } else {
            is_local_file = true;
            caps_urls.push(cmdline.address_url.clone());
        }
    }
    let caps_url: String = match caps_urls.get(0) {
        Some(u) => u.clone(),
        None => return Err("caps_url is none".to_string())
    };
    if is_local_file {
        let mut file = File::open(caps_url.clone()).map_err(|_| format!("failed to open file '{}'", caps_url))?;
        file.read_to_string(&mut buffer).map_err(|_| format!("failed to read file '{}'", caps_url))?;
    }

    let http_client = global_context.read().await.http_client.clone();
    let api_key = cmdline.api_key.clone();
    let mut headers = reqwest::header::HeaderMap::new();
    if !api_key.is_empty() {
        headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(format!("Bearer {}", api_key).as_str()).unwrap());
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_str(format!("refact-lsp {}", crate::version::build_info::PKG_VERSION).as_str()).unwrap());
    }
    let mut r1_mb_error_text = "".to_string();

    if is_remote_address {
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
            return Err(format!("cannot fetch caps, status={}", status));
        }
    }

    let r1_mb: Option<CodeAssistantCaps> = match serde_json::from_str(&buffer) {
        Ok(v) => v,
        Err(e) => {
            r1_mb_error_text = format!("{}", e);
            None
        }
    };
    let mut r1 = r1_mb.ok_or(format!("failed to parse caps: {}", r1_mb_error_text))?;

    let r0: ModelsOnly = serde_json::from_str(&KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;
    _inherit_r1_from_r0(&mut r1, &r0);
    apply_models_dict_patch(&mut r1);
    r1.endpoint_template = relative_to_full_url(&caps_url, &r1.endpoint_template)?;
    r1.endpoint_chat_passthrough = relative_to_full_url(&caps_url, &r1.endpoint_chat_passthrough)?;
    r1.telemetry_basic_dest = relative_to_full_url(&caps_url, &r1.telemetry_basic_dest)?;
    r1.telemetry_corrected_snippets_dest = relative_to_full_url(&caps_url, &r1.telemetry_corrected_snippets_dest)?;
    r1.telemetry_basic_retrieve_my_own = relative_to_full_url(&caps_url, &r1.telemetry_basic_retrieve_my_own)?;
    r1.endpoint_embeddings_template = relative_to_full_url(&caps_url, &r1.endpoint_embeddings_template)?;
    r1.tokenizer_path_template = relative_to_full_url(&caps_url, &r1.tokenizer_path_template)?;
    if r1.embedding_n_ctx == 0 {
        r1.embedding_n_ctx = 512;
    }

    info!("caps {} completion models", r1.code_completion_models.len());
    info!("caps default completion model: \"{}\"", r1.code_completion_default_model);
    info!("caps {} chat models", r1.code_chat_models.len());
    info!("caps default chat model: \"{}\"", r1.code_chat_default_model);
    // info!("running models: {:?}", r1.running_models);
    // info!("code_chat_models models: {:?}", r1.code_chat_models);
    // info!("code completion models: {:?}", r1.code_completion_models);
    Ok(Arc::new(StdRwLock::new(r1)))
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
        if !r1.code_completion_models.contains_key(k) && !r1.code_chat_models.contains_key(k) && *k != r1.default_embeddings_model {
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
    if let Some(model_rec) = models.get(&strip_model_from_finetune(&take_this_one.to_string())) {
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

const HF_DEFAULT_CAPS: &str = r#"
{
    "cloud_name": "Hugging Face",
    "endpoint_template": "https://api-inference.huggingface.co/models/$MODEL",
    "endpoint_style": "hf",
    "tokenizer_path_template": "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
    "tokenizer_rewrite_path": {
        "meta-llama/Llama-2-70b-chat-hf": "TheBloke/Llama-2-70B-fp16"
    },
    "code_completion_default_model": "bigcode/starcoder",
    "code_completion_n_ctx": 2048,
    "code_chat_default_model": "meta-llama/Llama-2-70b-chat-hf",
    "telemetry_basic_dest": "https://staging.smallcloud.ai/v1/telemetry-basic",
    "telemetry_corrected_snippets_dest": "https://www.smallcloud.ai/v1/feedback",
    "running_models": ["bigcode/starcoder", "meta-llama/Llama-2-70b-chat-hf"]
}
"#;
