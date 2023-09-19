use tracing::{info, error};
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::path::PathBuf;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use url::Url;

const CAPS_FILENAME: &str = "coding_assistant_caps.json";


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_stop: bool,
    pub supports_scratchpads: HashMap<String, serde_json::Value>,
    pub default_scratchpad: String,
    #[serde(default)]
    pub similar_models: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantCaps {
    pub cloud_name: String,
    pub endpoint_template: String,
    pub endpoint_style: String,
    pub tokenizer_path_template: String,
    pub tokenizer_rewrite_path: HashMap<String, String>,
    pub telemetry_basic_dest: String,
    #[serde(default)]
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_completion_default_model: String,
    #[serde(default)]
    pub code_chat_models: HashMap<String, ModelRecord>,
    pub code_chat_default_model: String,
    pub running_models: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelsOnly {
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_chat_models: HashMap<String, ModelRecord>,
}

const KNOWN_MODELS: &str = r#"
{
    "code_completion_models": {
        "bigcode/starcoder": {
            "n_ctx": 4096,
            "supports_stop": true,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-PSM",
            "similar_models": ["bigcode/starcoderbase"]
        },
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_stop": true,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-PSM"
        },
        "codellama/CodeLlama-13b-hf": {
            "n_ctx": 4096,
            "supports_stop": true,
            "supports_scratchpads": {
                "FIM-PSM": {"prefix_token": "<PRE>", "suffix_token": "<SUF>", "middle_token": "<MID>", "eot_token": "<EOT>"}
            },
            "default_scratchpad": "FIM-PSM"
        }
    },
    "code_chat_models": {
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "<empty_output>",
                    "keyword_system": "SYSTEM",
                    "keyword_user": "USER",
                    "keyword_assistant": "ASSISTANT",
                    "stop_list": ["<empty_output>"],
                    "default_system_message": "You are a programming assistant."
                }
            },
            "default_scratchpad": "CHAT-GENERIC"
        },
        "llama2/7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            },
            "default_scratchpad": "CHAT-LLAMA2",
            "similar_models": ["llama2/13b"]
        },
        "meta-llama/Llama-2-70b-chat-hf": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            },
            "default_scratchpad": "CHAT-LLAMA2"
        }
    }
}
"#;

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
    "code_chat_default_model": "",
    "telemetry_basic_dest": "https://www.smallcloud.ai/v1/usage-stats",
    "telemetry_corrected_snippets_dest": "https://www.smallcloud.ai/v1/feedback",
    "running_models": ["bigcode/starcoder", "meta-llama/Llama-2-70b-chat-hf"]
}
"#;

const SMC_DEFAULT_CAPS: &str = r#"
{
    "cloud_name": "Refact",
    "endpoint_template": "https://inference.smallcloud.ai/v1/completions",
    "endpoint_style": "openai",
    "code_completion_default_model": "smallcloudai/Refact-1_6B-fim",
    "code_chat_default_model": "smallcloudai/Refact-1_6B-fim",
    "tokenizer_path_template": "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
    "tokenizer_rewrite_path": {},
    "running_models": ["smallcloudai/Refact-1_6B-fim"]
}
"#;

pub async fn load_recommendations(
    cmdline: crate::global_context::CommandLine,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let mut buffer = String::new();
    let not_http = !cmdline.address_url.starts_with("http");
    let report_url: String;
    if cmdline.address_url == "HF" {
        buffer = HF_DEFAULT_CAPS.to_string();
        report_url = "<compiled-in-caps-hf>".to_string();
    } else if cmdline.address_url == "Refact" {
        buffer = SMC_DEFAULT_CAPS.to_string();
        report_url = "<compiled-in-caps-smc>".to_string();
    } else if not_http {
        let base: PathBuf = PathBuf::from(cmdline.address_url.clone());
        let file_path = base.join(CAPS_FILENAME);
        let mut file = File::open(file_path.clone()).map_err(|_| format!("failed to open file {:?}", file_path))?;
        file.read_to_string(&mut buffer).map_err(|_| format!("failed to read file {:?}", file_path))?;
        report_url = file_path.to_str().unwrap().to_string();
    } else {
        let base_url = Url::parse(&cmdline.address_url.clone()).map_err(|_| "failed to parse address url (1)".to_string())?;
        let joined_url = base_url.join(&CAPS_FILENAME).map_err(|_| "failed to parse address url (2)".to_string())?;
        report_url = joined_url.to_string();
        let http_client = reqwest::Client::new();
        let response = http_client.get(joined_url).send().await.map_err(|e| format!("{}", e))?;
        let status = response.status().as_u16();
        buffer = response.text().await.map_err(|e| format!("failed to read response: {}", e))?;
        if status != 200 {
            return Err(format!("server responded with: {:?}", buffer));
        }
    }
    info!("reading caps from {}", report_url);
    let r0: ModelsOnly = serde_json::from_str(&KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;
    let mut r1: CodeAssistantCaps = serde_json::from_str(&buffer).map_err(|e| {
        let up_to_line = buffer.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse {}: {}", up_to_line, report_url, e);
        format!("failed to parse {}: {}", report_url, e)
    })?;
    // inherit models from r0, only if not already present in r1
    for k in r0.code_completion_models.keys() {
        if !r1.code_completion_models.contains_key(k) {
            r1.code_completion_models.insert(k.to_string(), r0.code_completion_models[k].clone());
        }
    }
    for k in r0.code_chat_models.keys() {
        if !r1.code_chat_models.contains_key(k) {
            r1.code_chat_models.insert(k.to_string(), r0.code_chat_models[k].clone());
        }
    }
    // clone "similar_models"
    let ccmodel_keys_copy = r1.code_completion_models.keys().cloned().collect::<Vec<String>>();
    for k in ccmodel_keys_copy {
        let model_rec = r1.code_completion_models[&k].clone();
        for similar_model in model_rec.similar_models.iter() {
            r1.code_completion_models.insert(similar_model.to_string(), model_rec.clone());
        }
    }
    let chatmodel_keys_copy = r1.code_chat_models.keys().cloned().collect::<Vec<String>>();
    for k in chatmodel_keys_copy {
        let model_rec = r1.code_chat_models[&k].clone();
        for similar_model in model_rec.similar_models.iter() {
            r1.code_chat_models.insert(similar_model.to_string(), model_rec.clone());
        }
    }
    r1.code_completion_models = r1.code_completion_models.into_iter().filter(|(k, _)| r1.running_models.contains(&k)).collect();
    r1.code_chat_models = r1.code_chat_models.into_iter().filter(|(k, _)| r1.running_models.contains(&k)).collect();
    // endpoint_template
    if !r1.endpoint_template.starts_with("http") {
        let joined_url = Url::parse(&cmdline.address_url.clone())
            .and_then(|base_url| base_url.join(&r1.endpoint_template))
            .map_err(|_| format!("failed to join URL \"{}\" and possibly relative \"{}\"", &cmdline.address_url, &r1.endpoint_template))?;
        r1.endpoint_template = joined_url.to_string();
        info!("endpoint_template relative path: {}", &r1.endpoint_template);
    }
    info!("caps {} completion models", r1.code_completion_models.len());
    info!("caps default completion model: \"{}\"", r1.code_completion_default_model);
    info!("caps {} chat models", r1.code_chat_models.len());
    info!("caps default chat model: \"{}\"", r1.code_chat_default_model);
    Ok(Arc::new(StdRwLock::new(r1)))
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
    if let Some(model_rec) = models.get(take_this_one) {
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
