use tracing::{info, error};
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

const CAPS_FILENAME: &str = "coding_assistant_caps.json";


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_scratchpads: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub default_scratchpad: String,
    #[serde(default)]
    pub similar_models: Vec<String>,
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
    pub telemetry_basic_dest: String,
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
    pub default_embeddings_model: String,
    #[serde(default)]
    pub endpoint_embeddings_template: String,
    #[serde(default)]
    pub endpoint_embeddings_style: String,
    #[serde(default)]
    pub size_embeddings: i32,
    pub running_models: Vec<String>,
    #[serde(default)]
    pub caps_version: i64,  // need to reload if it increases on server, that happens when server configuration changes
    #[serde(default)]
    pub chat_rag_functions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelsOnly {
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_chat_models: HashMap<String, ModelRecord>,
}

const KNOWN_MODELS: &str = r####"
{
    "code_completion_models": {
        "bigcode/starcoder": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-PSM",
            "similar_models": [
                "bigcode/starcoderbase",
                "starcoder/15b/base",
                "starcoder/15b/plus",
                "starcoder/1b/base",
                "starcoder/3b/base",
                "starcoder/7b/base",
                "wizardcoder/15b",
                "starcoder/1b/vllm",
                "starcoder/3b/vllm",
                "starcoder/7b/vllm"
            ]
        },
        "smallcloudai/Refact-1_6B-fim": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {},
                "FIM-SPM": {}
            },
            "default_scratchpad": "FIM-SPM",
            "similar_models": [
                "Refact/1.6B",
                "Refact/1.6B/vllm"
            ]
        },
        "codellama/CodeLlama-13b-hf": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "fim_prefix": "<PRE>",
                    "fim_suffix": "<SUF>",
                    "fim_middle": "<MID>",
                    "eot": "<EOT>",
                    "eos": "</s>"
                }
            },
            "default_scratchpad": "FIM-PSM",
            "similar_models": [
                "codellama/7b"
            ]
        },
        "deepseek-coder/1.3b/base": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "FIM-PSM": {
                    "fim_prefix": "<｜fim▁begin｜>",
                    "fim_suffix": "<｜fim▁hole｜>",
                    "fim_middle": "<｜fim▁end｜>",
                    "eot": "<|EOT|>"
                }
            },
            "default_scratchpad": "FIM-PSM",
            "similar_models": [
                "deepseek-coder/5.7b/mqa-base",
                "deepseek-coder/1.3b/vllm",
                "deepseek-coder/5.7b/vllm"
            ]
        }
    },
    "code_chat_models": {
        "meta-llama/Llama-2-70b-chat-hf": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            }
        },
        "gpt-3.5-turbo": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
            ]
        },
        "gpt-4": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "PASSTHROUGH": {
                    "default_system_message": "You are a coding assistant that outputs short answers, gives links to documentation."
                }
            },
            "similar_models": [
            ]
        },
        "starchat/15b/beta": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "<|system|>\n",
                    "keyword_user": "<|end|>\n<|user|>\n",
                    "keyword_assistant": "<|end|>\n<|assistant|>\n",
                    "stop_list": [
                        "<|system|>",
                        "<|user|>",
                        "<|assistant|>",
                        "<|end|>",
                        "<empty_output>"
                    ],
                    "default_system_message": "You are a programming assistant."
                }
            }
        },
        "llama2/7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-LLAMA2": {
                    "default_system_message": "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe. Please ensure that your responses are socially unbiased and positive in nature. If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information."
                }
            },
            "similar_models": [
                "llama2/13b"
            ]
        },
        "wizardlm/7b": {
            "n_ctx": 2048,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "<s>",
                    "keyword_user": "\nUSER: ",
                    "keyword_assistant": "\nASSISTANT: ",
                    "eot": "",
                    "stop_list": ["\n\n"],
                    "default_system_message": "You are a helpful AI assistant.\n"
                }
            },
            "similar_models": [
                "wizardlm/13b",
                "wizardlm/30b"
            ]
        },
        "magicoder/6.7b": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "\n@@ Instruction\n",
                    "keyword_assistant": "\n@@ Response\n",
                    "stop_list": [],
                    "default_system_message": "You are an exceptionally intelligent coding assistant that consistently delivers accurate and reliable responses to user instructions.",
                    "eot": "<|EOT|>"
                }
            }
        },
        "mistral/7b/instruct-v0.1": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "[INST] ",
                    "keyword_assistant": "[/INST]\n",
                    "stop_list": [],
                    "default_system_message": "",
                    "eot": "</s>"
                }
            },
            "similar_models": [
                "mixtral/8x7b/instruct-v0.1"
            ]
        },
        "phind/34b/v2": {
            "n_ctx": 4095,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "### System Prompt\n",
                    "keyword_user": "\n### User Message\n",
                    "keyword_assistant": "\n### Assistant\n",
                    "stop_list": [],
                    "default_system_message": "You are an intelligent programming assistant.",
                    "eot": "</s>"
                }
            }
        },
        "deepseek-coder/6.7b/instruct": {
            "n_ctx": 4096,
            "supports_scratchpads": {
                "CHAT-GENERIC": {
                    "token_esc": "",
                    "keyword_system": "",
                    "keyword_user": "### Instruction:\n",
                    "keyword_assistant": "### Response:\n",
                    "stop_list": [],
                    "default_system_message": "You are an AI programming assistant, utilizing the Deepseek Coder model, developed by Deepseek Company, and you only answer questions related to computer science. For politically sensitive questions, security and privacy issues, and other non-computer science questions, you will refuse to answer.",
                    "eot": "<|EOT|>"
                }
            },
            "similar_models": [
                "deepseek-coder/33b/instruct"
            ]
        }
    }
}
"####;

const HF_DEFAULT_CAPS: &str = r#"
{
    "cloud_name": "Hugging Face",
    "endpoint_template": "https://api-inference.huggingface.co/models/$MODEL",
    "endpoint_style": "hf",

    "default_embeddings_model": "BAAI/bge-small-en-v1.5",
    "endpoint_embeddings_template": "https://api-inference.huggingface.co/models/$MODEL",
    "endpoint_embeddings_style": "hf",
    "size_embeddings": 384,

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

pub async fn load_caps(
    cmdline: crate::global_context::CommandLine,
    global_context: Arc<RwLock<GlobalContext>>,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
    let mut buffer = String::new();
    let mut is_local_file = false;
    let mut is_remote_address = false;
    let caps_url: String;
    if cmdline.address_url == "Refact" {
        is_remote_address = true;
        caps_url = "https://inference.smallcloud.ai/coding_assistant_caps.json".to_string();
    } else if cmdline.address_url == "HF" {
        buffer = HF_DEFAULT_CAPS.to_string();
        caps_url = "<compiled-in-caps-hf>".to_string();
    } else {
        if cmdline.address_url.starts_with("http") {
            is_remote_address = true;
            let base_url = Url::parse(&cmdline.address_url.clone()).map_err(|_| "failed to parse address url (1)".to_string())?;
            let joined_url = base_url.join(&CAPS_FILENAME).map_err(|_| "failed to parse address url (2)".to_string())?;
            caps_url = joined_url.to_string();
        } else {
            is_local_file = true;
            caps_url = cmdline.address_url.clone();
        }
    }
    if is_local_file {
        let mut file = File::open(caps_url.clone()).map_err(|_| format!("failed to open file '{}'", caps_url))?;
        file.read_to_string(&mut buffer).map_err(|_| format!("failed to read file '{}'", caps_url))?;
    }
    if is_remote_address {
        let api_key = cmdline.api_key.clone();
        let http_client = global_context.read().await.http_client.clone();
        let mut headers = reqwest::header::HeaderMap::new();
        if !api_key.is_empty() {
            headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(format!("Bearer {}", api_key).as_str()).unwrap());
        }
        let response = http_client.get(caps_url.clone()).headers(headers).send().await.map_err(|e| format!("{}", e))?;
        let status = response.status().as_u16();
        buffer = response.text().await.map_err(|e| format!("failed to read response: {}", e))?;
        if status != 200 {
            return Err(format!("server responded with: {}", buffer));
        }
    }
    info!("reading caps from {}", caps_url);
    let r0: ModelsOnly = serde_json::from_str(&KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;
    let mut r1: CodeAssistantCaps = serde_json::from_str(&buffer).map_err(|e| {
        let up_to_line = buffer.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        error!("{}\nfailed to parse {}: {}", up_to_line, caps_url, e);
        format!("failed to parse {}: {}", caps_url, e)
    })?;
    _inherit_r1_from_r0(&mut r1, &r0);
    r1.endpoint_template = relative_to_full_url(&caps_url, &r1.endpoint_template)?;
    r1.endpoint_chat_passthrough = relative_to_full_url(&caps_url, &r1.endpoint_chat_passthrough)?;
    r1.telemetry_basic_dest = relative_to_full_url(&caps_url, &r1.telemetry_basic_dest)?;
    r1.telemetry_corrected_snippets_dest = relative_to_full_url(&caps_url, &r1.telemetry_corrected_snippets_dest)?;
    r1.endpoint_embeddings_template = relative_to_full_url(&caps_url, &r1.endpoint_embeddings_template)?;
    info!("caps {} completion models", r1.code_completion_models.len());
    info!("caps default completion model: \"{}\"", r1.code_completion_default_model);
    info!("caps {} chat models", r1.code_chat_models.len());
    info!("caps default chat model: \"{}\"", r1.code_chat_default_model);
    Ok(Arc::new(StdRwLock::new(r1)))
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

fn _inherit_r1_from_r0(
    r1: &mut CodeAssistantCaps,
    r0: &ModelsOnly,
) {
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
    // clone to "similar_models"
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
    r1.code_completion_models = r1.code_completion_models.clone().into_iter().filter(|(k, _)| r1.running_models.contains(&k)).collect();
    r1.code_chat_models = r1.code_chat_models.clone().into_iter().filter(|(k, _)| r1.running_models.contains(&k)).collect();

    for k in r1.running_models.iter() {
        if !r1.code_completion_models.contains_key(k) && !r1.code_chat_models.contains_key(k) {
            info!("indicated as running, unknown model {}", k);
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
