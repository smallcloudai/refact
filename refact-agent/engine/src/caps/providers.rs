use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use structopt::StructOpt;

use crate::caps::{
    BaseModelRecord, ChatModelRecord, CodeAssistantCaps, CompletionModelRecord, DefaultModels,
    EmbeddingModelRecord, HasBaseModelRecord, default_embedding_batch, default_rejection_threshold,
    load_caps_value_from_url, resolve_relative_urls, strip_model_from_finetune, normalize_string
};
use crate::custom_error::{MapErrToString, YamlError};
use crate::global_context::{CommandLine, GlobalContext};
use crate::caps::self_hosted::SelfHostedCaps;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CapsProvider {
    #[serde(alias = "cloud_name", default, deserialize_with = "normalize_string")]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub supports_completion: bool,

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

    #[serde(default)]
    pub tokenizer_api_key: String,

    #[serde(default)]
    pub code_completion_n_ctx: usize,

    #[serde(default)]
    pub support_metadata: bool,

    #[serde(default)]
    pub completion_models: IndexMap<String, CompletionModelRecord>,
    #[serde(default)]
    pub chat_models: IndexMap<String, ChatModelRecord>,
    #[serde(default, alias = "default_embeddings_model")]
    pub embedding_model: EmbeddingModelRecord,

    #[serde(default)]
    pub models_dict_patch: IndexMap<String, serde_json::Value>, // Used to patch some params from cloud, like n_ctx for pro/free users

    #[serde(flatten)]
    pub defaults: DefaultModels,

    #[serde(default)]
    pub running_models: Vec<String>,
}

impl CapsProvider {
    pub fn apply_override(&mut self, value: serde_yaml::Value) -> Result<(), String> {
        set_field_if_exists::<bool>(&mut self.enabled, "enabled", &value)?;
        set_field_if_exists::<String>(&mut self.endpoint_style, "endpoint_style", &value)?;
        set_field_if_exists::<String>(&mut self.completion_endpoint, "completion_endpoint", &value)?;
        set_field_if_exists::<String>(&mut self.chat_endpoint, "chat_endpoint", &value)?;
        set_field_if_exists::<String>(&mut self.embedding_endpoint, "embedding_endpoint", &value)?;
        set_field_if_exists::<String>(&mut self.api_key, "api_key", &value)?;
        set_field_if_exists::<String>(&mut self.tokenizer_api_key, "tokenizer_api_key", &value)?;
        set_field_if_exists::<EmbeddingModelRecord>(&mut self.embedding_model, "embedding_model", &value)?;
        if value.get("embedding_model").is_some() {
            self.embedding_model.base.removable = true;
            self.embedding_model.base.user_configured = true;
        }

        extend_model_collection::<ChatModelRecord>(&mut self.chat_models, "chat_models", &value, &self.running_models)?;
        extend_model_collection::<CompletionModelRecord>(&mut self.completion_models, "completion_models", &value, &self.running_models)?;
        extend_collection::<Vec<String>>(&mut self.running_models, "running_models", &value)?;

        match serde_yaml::from_value::<DefaultModels>(value) {
            Ok(default_models) => {
                self.defaults.apply_override(&default_models, None);
            },
            Err(e) => return Err(e.to_string()),
        }

        Ok(())
    }
}

fn set_field_if_exists<T: for<'de> serde::Deserialize<'de>>(
    target: &mut T, field: &str, value: &serde_yaml::Value
) -> Result<(), String> {
    if let Some(val) = value.get(field) {
        *target = serde_yaml::from_value(val.clone())
            .map_err(|_| format!("Field '{}' has incorrect type", field))?;
    }
    Ok(())
}

fn extend_collection<C: for<'de> serde::Deserialize<'de> + Extend<C::Item> + IntoIterator>(
    target: &mut C, field: &str, value: &serde_yaml::Value
) -> Result<(), String> {
    if let Some(value) = value.get(field) {
        let imported_collection = serde_yaml::from_value::<C>(value.clone())
            .map_err(|_| format!("Invalid format for {field}"))?;

        target.extend(imported_collection);
    }
    Ok(())
}

// Special implementation for ChatModelRecord and CompletionModelRecord collections
// that sets removable=true for newly added models
fn extend_model_collection<T: for<'de> serde::Deserialize<'de> + HasBaseModelRecord>(
    target: &mut IndexMap<String, T>, field: &str, value: &serde_yaml::Value, prev_running_models: &Vec<String>
) -> Result<(), String> {
    if let Some(value) = value.get(field) {
        let imported_collection = serde_yaml::from_value::<IndexMap<String, T>>(value.clone())
            .map_err(|_| format!("Invalid format for {field}"))?;

        for (key, mut model) in imported_collection {
            model.base_mut().user_configured = true;
            if !target.contains_key(&key) && !prev_running_models.contains(&key) {
                model.base_mut().removable = true;
            }
            target.insert(key, model);
        }
    }
    Ok(())
}

fn default_endpoint_style() -> String { "openai".to_string() }

fn default_true() -> bool { true }

impl<'de> serde::Deserialize<'de> for EmbeddingModelRecord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            String(String),
            Full(EmbeddingModelRecordHelper),
        }

        #[derive(Deserialize)]
        struct EmbeddingModelRecordHelper {
            #[serde(flatten)]
            base: BaseModelRecord,
            #[serde(default)]
            embedding_size: i32,
            #[serde(default = "default_rejection_threshold")]
            rejection_threshold: f32,
            #[serde(default = "default_embedding_batch")]
            embedding_batch: usize,
        }

        match Input::deserialize(deserializer)? {
            Input::String(name) => Ok(EmbeddingModelRecord {
                base: BaseModelRecord { name, ..Default::default() },
                ..Default::default()
            }),
            Input::Full(mut helper) => {
                if helper.embedding_batch > 256 {
                    tracing::warn!("embedding_batch can't be higher than 256");
                    helper.embedding_batch = default_embedding_batch();
                }

                Ok(EmbeddingModelRecord {
                    base: helper.base,
                    embedding_batch: helper.embedding_batch,
                    rejection_threshold: helper.rejection_threshold,
                    embedding_size: helper.embedding_size,
                })
            },
        }
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct ModelDefaultSettingsUI {
    #[serde(default)]
    pub chat: ChatModelRecord,
    #[serde(default)]
    pub completion: CompletionModelRecord,
    #[serde(default)]
    pub embedding: EmbeddingModelRecord,
}

const PROVIDER_TEMPLATES: &[(&str, &str)] = &[
    ("anthropic", include_str!("../yaml_configs/default_providers/anthropic.yaml")),
    ("custom", include_str!("../yaml_configs/default_providers/custom.yaml")),
    ("deepseek", include_str!("../yaml_configs/default_providers/deepseek.yaml")),
    ("google_gemini", include_str!("../yaml_configs/default_providers/google_gemini.yaml")),
    ("groq", include_str!("../yaml_configs/default_providers/groq.yaml")),
    ("lmstudio", include_str!("../yaml_configs/default_providers/lmstudio.yaml")),
    ("ollama", include_str!("../yaml_configs/default_providers/ollama.yaml")),
    ("openai", include_str!("../yaml_configs/default_providers/openai.yaml")),
    ("openrouter", include_str!("../yaml_configs/default_providers/openrouter.yaml")),
    ("xai", include_str!("../yaml_configs/default_providers/xai.yaml")),
];
static PARSED_PROVIDERS: OnceLock<IndexMap<String, CapsProvider>> = OnceLock::new();
static PARSED_MODEL_DEFAULTS: OnceLock<IndexMap<String, ModelDefaultSettingsUI>> = OnceLock::new();

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

pub fn get_provider_model_default_settings_ui() -> &'static IndexMap<String, ModelDefaultSettingsUI> {
    PARSED_MODEL_DEFAULTS.get_or_init(|| {
        let mut map = IndexMap::new();
        for (name, yaml) in PROVIDER_TEMPLATES {
            let yaml_value = serde_yaml::from_str::<serde_yaml::Value>(yaml)
                .unwrap_or_else(|_| panic!("Failed to parse YAML for provider {}", name));

            let model_default_settings_ui_value = yaml_value.get("model_default_settings_ui").cloned()
                .expect(&format!("Missing `model_model_default_settings_ui` for provider template {name}"));
            let model_default_settings_ui = serde_yaml::from_value(model_default_settings_ui_value)
                .unwrap_or_else(|e| panic!("Failed to parse model_defaults for provider {}: {}", name, e));

            map.insert(name.to_string(), model_default_settings_ui);
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

pub fn post_process_provider(
    provider: &mut CapsProvider,
    include_disabled_models: bool,
    experimental: bool,
) {
    add_running_models(provider);
    populate_model_records(provider, experimental);
    apply_models_dict_patch(provider);
    add_name_and_id_to_model_records(provider);
    if !include_disabled_models {
        provider.chat_models.retain(|_, model| model.base.enabled);
        provider.completion_models.retain(|_, model| model.base.enabled);
    }
}

pub async fn read_providers_d(
    prev_providers: Vec<CapsProvider>,
    config_dir: &Path,
    experimental: bool,
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

    let provider_templates = get_provider_templates();

    for yaml_path in yaml_paths {
        let provider_name = match yaml_path.file_stem() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        if provider_templates.contains_key(&provider_name) {
            match get_provider_from_template_and_config_file(config_dir, &provider_name, false, false, experimental).await {
                Ok(provider) => {
                    providers.push(provider);
                },
                Err(e) => {
                    error_log.push(YamlError {
                        path: yaml_path.to_string_lossy().to_string(),
                        error_line: 0,
                        error_msg: e,
                    });
                }
            }
        } else {
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
            providers.push(provider);
        }
    }

    (providers, error_log)
}

fn add_running_models(provider: &mut CapsProvider) {
    let models_to_add = vec![
        &provider.defaults.chat_default_model,
        &provider.defaults.chat_light_model,
        &provider.defaults.chat_thinking_model,
        &provider.defaults.completion_default_model,
    ];

    for model in models_to_add {
        if !model.is_empty() && !provider.running_models.contains(model) {
            provider.running_models.push(model.clone());
        }
    }
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
        base_model_rec.tokenizer_api_key = provider.tokenizer_api_key.clone();
        base_model_rec.endpoint = endpoint.replace("$MODEL", model_name);
        base_model_rec.support_metadata = provider.support_metadata;
        base_model_rec.endpoint_style = provider.endpoint_style.clone();
    }

    for mut provider in providers {

        let completion_models = std::mem::take(&mut provider.completion_models);
        for (model_name, mut model_rec) in completion_models {
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
            if model_rec.base.endpoint.is_empty() {
                add_provider_details_to_model(
                    &mut model_rec.base, &provider, &model_name, &provider.chat_endpoint
                );
            }

            caps.chat_models.insert(model_rec.base.id.clone(), Arc::new(model_rec));
        }

        if provider.embedding_model.is_configured() && provider.embedding_model.base.enabled {
            let mut embedding_model = std::mem::take(&mut provider.embedding_model);

            if embedding_model.base.endpoint.is_empty() {
                let model_name = embedding_model.base.name.clone();
                add_provider_details_to_model(
                    &mut embedding_model.base, &provider, &model_name, &provider.embedding_endpoint
                );
            }
            caps.embedding_model = embedding_model;
        }

        caps.defaults.apply_override(&provider.defaults, Some(&provider.name));
    }
}

fn add_name_and_id_to_model_records(provider: &mut CapsProvider) {
    for (model_name, model_rec) in &mut provider.completion_models {
        model_rec.base.name = model_name.to_string();
        model_rec.base.id = format!("{}/{}", provider.name, model_name);
    }

    for (model_name, model_rec) in &mut provider.chat_models {
        model_rec.base.name = model_name.to_string();
        model_rec.base.id = format!("{}/{}", provider.name, model_name);
    }

    if provider.embedding_model.is_configured() {
        provider.embedding_model.base.id = format!("{}/{}", provider.name, provider.embedding_model.base.name);
    }
}

fn apply_models_dict_patch(provider: &mut CapsProvider) {
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

#[derive(Deserialize)]
pub struct KnownModels {
    pub completion_models: IndexMap<String, CompletionModelRecord>,
    pub chat_models: IndexMap<String, ChatModelRecord>,
    pub embedding_models: IndexMap<String, EmbeddingModelRecord>,
}
const UNPARSED_KNOWN_MODELS: &'static str = include_str!("../known_models.json");
static KNOWN_MODELS: OnceLock<KnownModels> = OnceLock::new();

pub fn get_known_models() -> &'static KnownModels {
    KNOWN_MODELS.get_or_init(|| {
        serde_json::from_str::<KnownModels>(UNPARSED_KNOWN_MODELS).map_err(|e| {
            let up_to_line = UNPARSED_KNOWN_MODELS.lines().take(e.line()).collect::<Vec<&str>>().join("\n");
            panic!("{}\nfailed to parse KNOWN_MODELS: {}", up_to_line, e);
        }).unwrap()
    })
}

fn populate_model_records(provider: &mut CapsProvider, experimental: bool) {
    let known_models = get_known_models();

    for model_name in &provider.running_models {
        if !provider.completion_models.contains_key(model_name) {
            if let Some(model_rec) = find_model_match(model_name, &provider.completion_models, &known_models.completion_models, experimental) {
                provider.completion_models.insert(model_name.clone(), model_rec);
            }
        }

        if !provider.chat_models.contains_key(model_name) {
            if let Some(model_rec) = find_model_match(model_name, &provider.chat_models, &known_models.chat_models, experimental) {
                provider.chat_models.insert(model_name.clone(), model_rec);
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
        if let Some(model_rec) = find_model_match(&model_name, &IndexMap::new(), &known_models.embedding_models, experimental) {
            provider.embedding_model = model_rec;
            provider.embedding_model.base.name = model_name;
        } else {
            tracing::warn!("Unknown embedding model '{}', maybe configure it or update this binary", model_name);
        }
    }
}

fn find_model_match<T: Clone + HasBaseModelRecord>(
    model_name: &String,
    provider_models: &IndexMap<String, T>,
    known_models: &IndexMap<String, T>,
    experimental: bool,
) -> Option<T> {
    let model_stripped = strip_model_from_finetune(model_name);

    if let Some(model) = provider_models.get(model_name)
        .or_else(|| provider_models.get(&model_stripped)) {
        if !model.base().experimental || experimental {
            return Some(model.clone());
        }
    }

    for model in provider_models.values() {
        if model.base().similar_models.contains(model_name) ||
            model.base().similar_models.contains(&model_stripped) {
            if !model.base().experimental || experimental {
                return Some(model.clone());
            }
        }
    }

    if let Some(model) = known_models.get(model_name)
        .or_else(|| known_models.get(&model_stripped)) {
        if !model.base().experimental || experimental {
            return Some(model.clone());
        }
    }

    for model in known_models.values() {
        if model.base().similar_models.contains(&model_name.to_string()) ||
            model.base().similar_models.contains(&model_stripped) {
            if !model.base().experimental || experimental {
                return Some(model.clone());
            }
        }
    }

    None
}

pub fn resolve_api_key(provider: &CapsProvider, key: &str, fallback: &str, key_name: &str) -> String {
    match key {
        k if k.is_empty() => fallback.to_string(),
        k if k.starts_with("$") => {
            match std::env::var(&k[1..]) {
                Ok(env_val) => env_val,
                Err(e) => {
                    tracing::error!(
                        "tried to read {} from env var {} for provider {}, but failed: {}",
                        key_name, k, provider.name, e
                    );
                    fallback.to_string()
                }
            }
        }
        k => k.to_string(),
    }
}

pub fn resolve_provider_api_key(provider: &CapsProvider, cmdline_api_key: &str) -> String {
    resolve_api_key(provider, &provider.api_key, &cmdline_api_key, "API key")
}

pub fn resolve_tokenizer_api_key(provider: &CapsProvider) -> String {
    resolve_api_key(provider, &provider.tokenizer_api_key, "", "tokenizer API key")
}

pub async fn get_provider_from_template_and_config_file(
    config_dir: &Path, name: &str, config_file_must_exist: bool, post_process: bool, experimental: bool
) -> Result<CapsProvider, String> {
    let mut provider = get_provider_templates().get(name).cloned()
        .ok_or("Provider template not found")?;

    let provider_path = config_dir.join("providers.d").join(format!("{name}.yaml"));
    let config_file_value = match tokio::fs::read_to_string(&provider_path).await {
        Ok(content) => {
            serde_yaml::from_str::<serde_yaml::Value>(&content)
                .map_err_with_prefix(format!("Error parsing file {}:", provider_path.display()))?
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !config_file_must_exist => {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        },
        Err(e) => {
            return Err(format!("Failed to read file {}: {}", provider_path.display(), e));
        }
    };

    provider.apply_override(config_file_value)?;

    if post_process {
        post_process_provider(&mut provider, true, experimental);
    }

    Ok(provider)
}

pub async fn get_provider_from_server(gcx: Arc<ARwLock<GlobalContext>>) -> Result<CapsProvider, String> {
    let command_line = CommandLine::from_args();
    let cmdline_api_key = command_line.api_key.clone();
    let cmdline_experimental = command_line.experimental;
    let (caps_value, caps_url) = load_caps_value_from_url(command_line, gcx.clone()).await?;

    if let Ok(self_hosted_caps) = serde_json::from_value::<SelfHostedCaps>(caps_value.clone()) {
        let mut provider = self_hosted_caps.into_provider(&caps_url, &cmdline_api_key)?;
        post_process_provider(&mut provider, true, cmdline_experimental);
        provider.api_key = resolve_provider_api_key(&provider, &cmdline_api_key);
        provider.tokenizer_api_key = resolve_tokenizer_api_key(&provider);
        Ok(provider)
    } else {
        let mut provider = serde_json::from_value::<CapsProvider>(caps_value).map_err_to_string()?;

        resolve_relative_urls(&mut provider, &caps_url)?;
        post_process_provider(&mut provider, true, cmdline_experimental);
        provider.api_key = resolve_provider_api_key(&provider, &cmdline_api_key);
        provider.tokenizer_api_key = resolve_tokenizer_api_key(&provider);
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_templates() {
        let _ = get_provider_templates(); // This will panic if any template fails to parse
    }

    #[test]
    fn test_parse_known_models() {
        let _ = get_known_models(); // This will panic if any model fails to parse
    }
}
