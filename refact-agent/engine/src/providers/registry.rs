use std::path::Path;

use crate::providers::traits::ProviderTrait;
use crate::providers::{
    refact::RefactProvider,
    anthropic::AnthropicProvider,
    openai::OpenAIProvider,
    openai_responses::OpenAIResponsesProvider,
    openai_codex::OpenAICodexProvider,
    openrouter::OpenRouterProvider,
    ollama::OllamaProvider,
    lmstudio::LMStudioProvider,
    vllm::VLLMProvider,
    groq::GroqProvider,
    deepseek::DeepseekProvider,
    minimax::MiniMaxProvider,
    xai::XAIProvider,
    xai_responses::XAIResponsesProvider,
    google_gemini::GoogleGeminiProvider,
    custom::CustomProvider,
    claude_code::ClaudeCodeProvider,
};

pub const PROVIDER_NAMES: &[&str] = &[
    "refact",
    "anthropic",
    "openai",
    "openai_responses",
    "openai_codex",
    "openrouter",
    "ollama",
    "lmstudio",
    "vllm",
    "groq",
    "deepseek",
    "minimax",
    "xai",
    "xai_responses",
    "google_gemini",
    "custom",
    "claude_code",
];

pub fn create_provider(name: &str) -> Option<Box<dyn ProviderTrait>> {
    match name {
        "refact" => Some(Box::new(RefactProvider::default())),
        "anthropic" => Some(Box::new(AnthropicProvider::default())),
        "openai" => Some(Box::new(OpenAIProvider::default())),
        "openai_responses" => Some(Box::new(OpenAIResponsesProvider::default())),
        "openai_codex" => Some(Box::new(OpenAICodexProvider::default())),
        "openrouter" => Some(Box::new(OpenRouterProvider::default())),
        "ollama" => Some(Box::new(OllamaProvider::default())),
        "lmstudio" => Some(Box::new(LMStudioProvider::default())),
        "vllm" => Some(Box::new(VLLMProvider::default())),
        "groq" => Some(Box::new(GroqProvider::default())),
        "deepseek" => Some(Box::new(DeepseekProvider::default())),
        "minimax" => Some(Box::new(MiniMaxProvider::default())),
        "xai" => Some(Box::new(XAIProvider::default())),
        "xai_responses" => Some(Box::new(XAIResponsesProvider::default())),
        "google_gemini" => Some(Box::new(GoogleGeminiProvider::default())),
        "custom" => Some(Box::new(CustomProvider::default())),
        "claude_code" => Some(Box::new(ClaudeCodeProvider::default())),
        _ => None,
    }
}

pub struct ProviderRegistry {
    providers: Vec<Box<dyn ProviderTrait>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add(&mut self, provider: Box<dyn ProviderTrait>) {
        let name = provider.name();
        self.providers.retain(|p| p.name() != name);
        self.providers.push(provider);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ProviderTrait> {
        self.providers.iter().find(|p| p.name() == name).map(|p| p.as_ref())
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn ProviderTrait>> {
        self.providers.iter_mut().find(|p| p.name() == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn ProviderTrait)> {
        self.providers.iter().map(|p| (p.name(), p.as_ref()))
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn load_providers_from_config(
    config_dir: &Path,
    refact_address_url: &str,
    refact_api_key: &str,
) -> Result<ProviderRegistry, String> {
    let mut registry = ProviderRegistry::new();

    let refact_provider = RefactProvider::from_cli(
        refact_address_url.to_string(),
        refact_api_key.to_string(),
    );
    registry.add(Box::new(refact_provider));

    let providers_dir = config_dir.join("providers.d");
    if !providers_dir.exists() {
        return Ok(registry);
    }

    let mut entries = match tokio::fs::read_dir(&providers_dir).await {
        Ok(e) => e,
        Err(_) => return Ok(registry),
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name == "defaults" || name == "refact" {
            continue;
        }

        let mut provider = match create_provider(name) {
            Some(p) => p,
            None => continue,
        };

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read provider config {}: {}", path.display(), e);
                continue;
            }
        };

        let yaml: serde_yaml::Value = match serde_yaml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse provider config {}: {}", path.display(), e);
                continue;
            }
        };

        if let Err(e) = provider.provider_settings_apply(yaml) {
            tracing::warn!("Failed to apply provider config {}: {}", path.display(), e);
            continue;
        }

        registry.add(provider);
    }

    Ok(registry)
}

pub async fn save_provider_config(
    config_dir: &Path,
    name: &str,
    settings: serde_yaml::Value,
) -> Result<(), String> {
    let providers_dir = config_dir.join("providers.d");
    tokio::fs::create_dir_all(&providers_dir)
        .await
        .map_err(|e| format!("Failed to create providers.d: {}", e))?;

    let path = providers_dir.join(format!("{}.yaml", name));
    let content = serde_yaml::to_string(&settings)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write config: {}", e))
}

pub async fn delete_provider_config(config_dir: &Path, name: &str) -> Result<(), String> {
    let path = config_dir.join("providers.d").join(format!("{}.yaml", name));
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("Failed to delete config: {}", e))
}


