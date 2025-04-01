use std::path::{Path, PathBuf}; 
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};

use crate::caps::{strip_model_from_finetune, BaseModelRecord, ChatModelRecord, 
    CodeAssistantCaps, CompletionModelRecord, DefaultModels, EmbeddingModelRecord};
use crate::custom_error::YamlError;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CapsProvider {
    #[serde(alias = "cloud_name", default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_endpoint_style")]
    pub endpoint_style: String,

    // This aliases are for backward compatibility with cloud and self-hosted caps
    #[serde(default, alias = "endpoint_template")]
    pub completion_endpoint: String,
    #[serde(default, alias = "endpoint_chat_passthrough")]
    pub chat_endpoint: String,
    #[serde(default, alias = "endpoint_embeddings_template")]
    pub embedding_endpoint: String,

    #[serde(default)]
    pub api_key: String,

    #[serde(default = "default_code_completion_n_ctx")]
    pub code_completion_n_ctx: usize,

    #[serde(default)]
    pub support_metadata: bool,

    #[serde(default)]
    pub completion_models: IndexMap<String, CompletionModelRecord>,
    #[serde(default)]
    pub chat_models: IndexMap<String, ChatModelRecord>,
    #[serde(default, alias = "default_embeddings_model", deserialize_with = "deserialize_embedding_model")]
    pub embedding_model: EmbeddingModelRecord,

    #[serde(default)]
    pub models_dict_patch: IndexMap<String, serde_json::Value>, // Used to patch some params from cloud, like n_ctx for pro/free users

    #[serde(flatten)]
    pub defaults: DefaultModels,

    #[serde(default)]
    pub running_models: Vec<String>,
}

fn default_endpoint_style() -> String { "openai".to_string() }

fn default_code_completion_n_ctx() -> usize { 2048 }

fn default_true() -> bool { true }

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

const PROVIDER_TEMPLATES: &[(&str, &str)] = &[
    ("openai", include_str!("yaml_configs/default_providers/openai.yaml")),
    ("openrouter", include_str!("yaml_configs/default_providers/openrouter.yaml")),
];
static PARSED_PROVIDERS: OnceLock<IndexMap<String, CapsProvider>> = OnceLock::new();

pub fn get_provider_templates() -> &'static IndexMap<String, CapsProvider> {
    PARSED_PROVIDERS.get_or_init(|| {
        let mut map = IndexMap::new();
        for (name, yaml) in PROVIDER_TEMPLATES {
            if let Ok(mut provider) = serde_yaml::from_str::<CapsProvider>(yaml) {
                provider.name = name.to_string();
                map.insert(name.to_string(), provider);
            } else {
                panic!("Failed to parse template for provider {}", name);
            }
        }
        map
    })
}

/// Returns yaml files from providers.d directory, and list of errors from reading 
/// directory or listing files
pub async fn get_provider_yaml_paths(config_dir: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let providers_dir = config_dir.join("providers.d");
    let mut yaml_paths = Vec::new();
    let mut errors = Vec::new();
    
    let mut entries = match tokio::fs::read_dir(&providers_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(format!("Failed to read providers directory: {e}"));
            return (yaml_paths, errors);
        }
    };
    
    while let Some(entry_result) = entries.next_entry().await.transpose() {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                
                if path.is_file() && 
                   path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                    yaml_paths.push(path);
                }
            },
            Err(e) => {
                errors.push(format!("Error reading directory entry: {e}"));
            }
        }
    }
    
    (yaml_paths, errors)
}

pub async fn read_providers_d(
    prev_providers: Vec<CapsProvider>, 
    config_dir: &Path
) -> (Vec<CapsProvider>, Vec<YamlError>) {
    let providers_dir = config_dir.join("providers.d");
    let mut providers = prev_providers;
    let mut error_log = Vec::new();

    let (yaml_paths, read_errors) = get_provider_yaml_paths(config_dir).await;
    for error in read_errors {
        error_log.push(YamlError { 
            path: providers_dir.to_string_lossy().to_string(), 
            error_line: 0, 
            error_msg: error.to_string(),
        });
    }

    for yaml_path in yaml_paths {
        let provider_name = match yaml_path.file_stem() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let content = match tokio::fs::read_to_string(&yaml_path).await {
            Ok(content) => content,
            Err(e) => {
                error_log.push(YamlError {
                    path: yaml_path.to_string_lossy().to_string(),
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
                    path: yaml_path.to_string_lossy().to_string(),
                    error_line: e.location().map_or(0, |loc| loc.line()),
                    error_msg: format!("Failed to parse YAML: {}", e),
                });
                continue;
            }
        };
        provider.name = provider_name;

        let mut models_to_add = vec![
            &provider.defaults.chat_default_model,
            &provider.defaults.completion_default_model,
        ];
        models_to_add.extend(provider.chat_models.keys());
        models_to_add.extend(provider.completion_models.keys());

        for model in models_to_add {
            if !model.is_empty() && !provider.running_models.contains(model) {
                provider.running_models.push(model.clone());
            }
        }

        providers.push(provider);
    }

    (providers, error_log)
}

/// Returns the latest modification timestamp in seconds of any YAML file in the providers.d directory
pub async fn get_latest_provider_mtime(config_dir: &Path) -> Option<u64> {
    let (yaml_paths, reading_errors) = get_provider_yaml_paths(config_dir).await;
    
    for error in reading_errors {
        tracing::error!("{error}");
    }
    
    let mut latest_mtime = None;
    for path in yaml_paths {
        match tokio::fs::metadata(&path).await {
            Ok(metadata) => {
                if let Ok(mtime) = metadata.modified() {
                    latest_mtime = match latest_mtime {
                        Some(current_latest) if mtime > current_latest => Some(mtime),
                        None => Some(mtime),
                        _ => latest_mtime,
                    };
                }
            },
            Err(e) => {
                tracing::error!("Failed to get metadata for {}: {}", path.display(), e);
            }
        }
    }

    latest_mtime.map(|mtime| mtime.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
}

pub fn add_models_to_caps(caps: &mut CodeAssistantCaps, providers: Vec<CapsProvider>) {
    fn add_provider_details_to_model(base_model_rec: &mut BaseModelRecord, provider: &CapsProvider, model_name: &str, endpoint: &str) {
        base_model_rec.api_key = provider.api_key.clone();
        base_model_rec.endpoint = endpoint.replace("$MODEL", model_name);
        base_model_rec.support_metadata = provider.support_metadata;
        base_model_rec.endpoint_style = provider.endpoint_style.clone();
    }
    
    for mut provider in providers {

        let completion_models = std::mem::take(&mut provider.completion_models);
        for (model_name, mut model_rec) in completion_models {
            model_rec.base.name = model_name.to_string();
            model_rec.base.id = format!("{}/{}", provider.name, model_name);

            if model_rec.base.endpoint.is_empty() {
                add_provider_details_to_model(
                    &mut model_rec.base, &provider, &model_name, &provider.completion_endpoint
                );

                if provider.code_completion_n_ctx > 0 && provider.code_completion_n_ctx < model_rec.base.n_ctx {
                    // model is capable of more, but we may limit it from server or provider, e.x. for latency
                    model_rec.base.n_ctx = provider.code_completion_n_ctx; 
                }
            }
            
            caps.completion_models.insert(model_rec.base.id.clone(), Arc::new(model_rec));
        }

        let chat_models = std::mem::take(&mut provider.chat_models);
        for (model_name, mut model_rec) in chat_models {
            model_rec.base.name = model_name.to_string();
            model_rec.base.id = format!("{}/{}", provider.name, model_name);

            if model_rec.base.endpoint.is_empty() {
                add_provider_details_to_model(
                    &mut model_rec.base, &provider, &model_name, &provider.chat_endpoint
                );
            }

            caps.chat_models.insert(model_rec.base.id.clone(), Arc::new(model_rec));
        }

        if provider.embedding_model.is_configured() {
            let mut embedding_model = std::mem::take(&mut provider.embedding_model);
            embedding_model.base.id = format!("{}/{}", provider.name, embedding_model.base.name);

            if embedding_model.base.endpoint.is_empty() {
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
            }
            caps.embedding_model = embedding_model;
        }

        if !provider.defaults.chat_default_model.is_empty() {
            caps.defaults.chat_default_model = format!("{}/{}", provider.name, provider.defaults.chat_default_model);
        }
        if !provider.defaults.completion_default_model.is_empty() {
            caps.defaults.completion_default_model = format!("{}/{}", provider.name, provider.defaults.completion_default_model);
        }
        if !provider.defaults.chat_thinking_model.is_empty() {
            caps.defaults.chat_thinking_model = format!("{}/{}", provider.name, provider.defaults.chat_thinking_model);
        }
        if !provider.defaults.chat_light_model.is_empty() {
           caps.defaults.chat_light_model = format!("{}/{}", provider.name, provider.defaults.chat_light_model);
        }
    }
}


pub fn apply_models_dict_patch(providers: &mut Vec<CapsProvider>) {
    for provider in providers {
        for (model_name, rec_patched) in provider.models_dict_patch.iter() {
            if let Some(completion_rec) = provider.completion_models.get_mut(model_name) {
                if let Some(n_ctx) = rec_patched.get("n_ctx").and_then(|v| v.as_u64()) {
                    completion_rec.base.n_ctx = n_ctx as usize;
                }
            }
            
            if let Some(chat_rec) = provider.chat_models.get_mut(model_name) {
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

pub fn populate_provider_model_records(providers: &mut Vec<CapsProvider>) -> Result<(), String> {
    #[derive(Deserialize)]
    struct KnownModels {
        completion_models: IndexMap<String, CompletionModelRecord>,
        chat_models: IndexMap<String, ChatModelRecord>,
        embedding_models: IndexMap<String, EmbeddingModelRecord>,
    }
    const KNOWN_MODELS: &'static str = include_str!("known_models.json");
    let known_models: KnownModels = serde_json::from_str(KNOWN_MODELS).map_err(|e| {
        let up_to_line = KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
        tracing::error!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        format!("failed to parse KNOWN_MODELS: {}", e)
    })?;

    for provider in providers {
        for model_name in &provider.running_models {
            let model_stripped = strip_model_from_finetune(model_name);

            if !provider.completion_models.contains_key(&model_stripped) {
                let models_to_try = provider.completion_models.iter()
                    .chain(&known_models.completion_models);
                
                for (candidate_model_name, candidate_model_rec) in models_to_try {
                    if candidate_model_name == &model_stripped || candidate_model_rec.base.similar_models.contains(&model_stripped) {
                        provider.completion_models.insert(model_name.clone(), candidate_model_rec.clone());
                        break;
                    }
                }
            }

            if !provider.chat_models.contains_key(&model_stripped) {
                let models_to_try = provider.chat_models.iter()
                    .chain(&known_models.chat_models);
                
                for (candidate_model_name, candidate_model_rec) in models_to_try {
                    if candidate_model_name == &model_stripped || candidate_model_rec.base.similar_models.contains(&model_stripped) {
                        provider.chat_models.insert(model_name.clone(), candidate_model_rec.clone());
                        break;
                    }
                }
            }
        }

        for model in &provider.running_models {
            if !provider.completion_models.contains_key(model) && 
                !provider.chat_models.contains_key(model) &&
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

pub fn resolve_provider_api_key(provider: &CapsProvider, cmdline_api_key: &str) -> String {
    match &provider.api_key {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_templates() {
        let _ = get_provider_templates(); // This will panic if any template fails to parse
    }
}