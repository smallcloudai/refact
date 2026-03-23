use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::llm::adapter::WireFormat;
use crate::providers::config::resolve_env_var;
use crate::providers::traits::{CustomModelConfig, ModelPricing, ModelSource, ProviderRuntime, ProviderTrait, parse_enabled_models, parse_custom_models, set_model_enabled_impl};
use crate::providers::pricing::minimax_pricing;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MiniMaxProvider {
    pub api_key: String,
    pub enabled: bool,
    #[serde(default)]
    pub enabled_models: Vec<String>,
    #[serde(default)]
    pub custom_models: HashMap<String, CustomModelConfig>,
}

#[async_trait]
impl ProviderTrait for MiniMaxProvider {
    fn name(&self) -> &'static str {
        "minimax"
    }

    fn display_name(&self) -> &'static str {
        "MiniMax"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn ProviderTrait> {
        Box::new(self.clone())
    }

    fn default_wire_format(&self) -> WireFormat {
        WireFormat::OpenaiChatCompletions
    }

    fn model_filter_regex(&self) -> Option<&'static str> {
        Some(r"(?i)^minimax-")
    }

    fn provider_schema(&self) -> &'static str {
        r#"
fields:
  api_key:
    f_type: string_long
    f_desc: "MiniMax API key from platform.minimaxi.com"
    f_placeholder: "eyJhbG..."
    f_label: "API Key"
    smartlinks:
      - sl_label: "Get API Key"
        sl_goto: "https://platform.minimaxi.com/user-center/basic-information/interface-key"
description: |
  MiniMax cloud models (M2.7, M2.5, M2.5-highspeed with 204K context).
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
"#
    }

    fn provider_settings_apply(&mut self, yaml: serde_yaml::Value) -> Result<(), String> {
        if let Some(api_key) = yaml.get("api_key").and_then(|v| v.as_str()) {
            if api_key != "***" {
                self.api_key = api_key.to_string();
            }
        }
        if let Some(enabled) = yaml.get("enabled").and_then(|v| v.as_bool()) {
            self.enabled = enabled;
        }
        parse_enabled_models(&yaml, &mut self.enabled_models);
        parse_custom_models(&yaml, &mut self.custom_models);
        Ok(())
    }

    fn provider_settings_as_json(&self) -> serde_json::Value {
        json!({
            "api_key": if self.api_key.is_empty() { "" } else { "***" },
            "enabled": self.enabled,
            "enabled_models": self.enabled_models,
            "custom_models": self.custom_models
        })
    }

    fn build_runtime(&self) -> Result<ProviderRuntime, String> {
        let api_key = resolve_env_var(&self.api_key, "", "minimax api_key");

        Ok(ProviderRuntime {
            name: self.name().to_string(),
            display_name: self.display_name().to_string(),
            enabled: self.enabled && !api_key.is_empty() && !self.enabled_models.is_empty(),
            readonly: false,
            wire_format: self.default_wire_format(),
            chat_endpoint: "https://api.minimax.io/v1/chat/completions".to_string(),
            completion_endpoint: String::new(),
            embedding_endpoint: String::new(),
            api_key,
            auth_token: String::new(),
            tokenizer_api_key: String::new(),
            extra_headers: HashMap::new(),
            support_metadata: false,
            chat_models: Vec::new(),
            completion_models: Vec::new(),
            embedding_model: None,
        })
    }

    fn has_credentials(&self) -> bool {
        let key = resolve_env_var(&self.api_key, "", "minimax api_key");
        !key.is_empty()
    }

    fn model_source(&self) -> ModelSource {
        ModelSource::ModelCaps
    }

    fn enabled_models(&self) -> &[String] {
        &self.enabled_models
    }

    fn custom_models(&self) -> &HashMap<String, CustomModelConfig> {
        &self.custom_models
    }

    fn set_model_enabled(&mut self, model_id: &str, enabled: bool) {
        set_model_enabled_impl(&mut self.enabled_models, model_id, enabled);
    }

    fn add_custom_model(&mut self, model_id: String, config: CustomModelConfig) {
        self.custom_models.insert(model_id, config);
    }

    fn remove_custom_model(&mut self, model_id: &str) -> bool {
        self.custom_models.remove(model_id).is_some()
    }

    fn model_pricing(&self, model_id: &str) -> Option<ModelPricing> {
        if let Some(config) = self.custom_models.get(model_id) {
            if config.pricing.is_some() {
                return config.pricing.clone();
            }
        }
        minimax_pricing(model_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = MiniMaxProvider::default();
        assert_eq!(provider.name(), "minimax");
        assert_eq!(provider.display_name(), "MiniMax");
    }

    #[test]
    fn test_wire_format() {
        let provider = MiniMaxProvider::default();
        assert!(matches!(provider.default_wire_format(), WireFormat::OpenaiChatCompletions));
    }

    #[test]
    fn test_model_filter_regex() {
        let provider = MiniMaxProvider::default();
        let pattern = provider.model_filter_regex().unwrap();
        let re = regex::Regex::new(pattern).unwrap();
        assert!(re.is_match("MiniMax-M2.7"));
        assert!(re.is_match("MiniMax-M2.5-highspeed"));
        assert!(re.is_match("minimax-m2.5"));
        assert!(!re.is_match("gpt-4o"));
        assert!(!re.is_match("deepseek-chat"));
    }

    #[test]
    fn test_settings_apply() {
        let mut provider = MiniMaxProvider::default();
        let yaml: serde_yaml::Value = serde_yaml::from_str(r#"
            api_key: "test-key-123"
            enabled: true
            enabled_models:
              - "MiniMax-M2.7"
              - "MiniMax-M2.5-highspeed"
        "#).unwrap();
        provider.provider_settings_apply(yaml).unwrap();
        assert_eq!(provider.api_key, "test-key-123");
        assert!(provider.enabled);
        assert_eq!(provider.enabled_models.len(), 2);
        assert!(provider.enabled_models.contains(&"MiniMax-M2.7".to_string()));
        assert!(provider.enabled_models.contains(&"MiniMax-M2.5-highspeed".to_string()));
    }

    #[test]
    fn test_settings_as_json_masks_api_key() {
        let provider = MiniMaxProvider {
            api_key: "secret-key".to_string(),
            enabled: true,
            enabled_models: vec!["MiniMax-M2.7".to_string()],
            custom_models: HashMap::new(),
        };
        let json = provider.provider_settings_as_json();
        assert_eq!(json["api_key"], "***");
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn test_settings_as_json_empty_key() {
        let provider = MiniMaxProvider::default();
        let json = provider.provider_settings_as_json();
        assert_eq!(json["api_key"], "");
    }

    #[test]
    fn test_build_runtime_endpoint() {
        let provider = MiniMaxProvider {
            api_key: "test-key".to_string(),
            enabled: true,
            enabled_models: vec!["MiniMax-M2.7".to_string()],
            custom_models: HashMap::new(),
        };
        let runtime = provider.build_runtime().unwrap();
        assert_eq!(runtime.chat_endpoint, "https://api.minimax.io/v1/chat/completions");
        assert!(runtime.completion_endpoint.is_empty());
        assert!(runtime.embedding_endpoint.is_empty());
        assert_eq!(runtime.name, "minimax");
        assert_eq!(runtime.display_name, "MiniMax");
    }

    #[test]
    fn test_build_runtime_disabled_without_key() {
        let provider = MiniMaxProvider {
            api_key: String::new(),
            enabled: true,
            enabled_models: vec!["MiniMax-M2.7".to_string()],
            custom_models: HashMap::new(),
        };
        let runtime = provider.build_runtime().unwrap();
        assert!(!runtime.enabled);
    }

    #[test]
    fn test_build_runtime_disabled_without_models() {
        let provider = MiniMaxProvider {
            api_key: "test-key".to_string(),
            enabled: true,
            enabled_models: Vec::new(),
            custom_models: HashMap::new(),
        };
        let runtime = provider.build_runtime().unwrap();
        assert!(!runtime.enabled);
    }

    #[test]
    fn test_model_source() {
        let provider = MiniMaxProvider::default();
        assert!(matches!(provider.model_source(), ModelSource::ModelCaps));
    }

    #[test]
    fn test_set_model_enabled() {
        let mut provider = MiniMaxProvider::default();
        provider.set_model_enabled("MiniMax-M2.7", true);
        assert!(provider.enabled_models.contains(&"MiniMax-M2.7".to_string()));
        provider.set_model_enabled("MiniMax-M2.7", false);
        assert!(!provider.enabled_models.contains(&"MiniMax-M2.7".to_string()));
    }

    #[test]
    fn test_custom_model_management() {
        let mut provider = MiniMaxProvider::default();
        let config = CustomModelConfig::default();
        provider.add_custom_model("custom-minimax".to_string(), config);
        assert!(provider.custom_models().contains_key("custom-minimax"));
        assert!(provider.remove_custom_model("custom-minimax"));
        assert!(!provider.custom_models().contains_key("custom-minimax"));
        assert!(!provider.remove_custom_model("nonexistent"));
    }

    #[test]
    fn test_pricing_m27() {
        let pricing = minimax_pricing("MiniMax-M2.7").unwrap();
        assert!(pricing.prompt > 0.0);
        assert!(pricing.generated > 0.0);
    }

    #[test]
    fn test_pricing_m25_highspeed() {
        let pricing = minimax_pricing("MiniMax-M2.5-highspeed").unwrap();
        assert!(pricing.prompt > 0.0);
        assert!(pricing.generated > 0.0);
    }

    #[test]
    fn test_pricing_m25() {
        let pricing = minimax_pricing("MiniMax-M2.5").unwrap();
        assert!(pricing.prompt > 0.0);
        assert!(pricing.generated > 0.0);
    }

    #[test]
    fn test_pricing_unknown() {
        assert!(minimax_pricing("unknown-model").is_none());
    }

    #[test]
    fn test_has_credentials_empty() {
        let provider = MiniMaxProvider::default();
        assert!(!provider.has_credentials());
    }

    #[test]
    fn test_schema_is_valid_yaml() {
        let provider = MiniMaxProvider::default();
        let schema = provider.provider_schema();
        let parsed: serde_yaml::Value = serde_yaml::from_str(schema).unwrap();
        assert!(parsed.get("fields").is_some());
        assert!(parsed.get("description").is_some());
        assert!(parsed.get("available").is_some());
    }

    #[test]
    fn test_clone_box() {
        let provider = MiniMaxProvider {
            api_key: "key".to_string(),
            enabled: true,
            enabled_models: vec!["MiniMax-M2.7".to_string()],
            custom_models: HashMap::new(),
        };
        let cloned = provider.clone_box();
        assert_eq!(cloned.name(), "minimax");
        assert_eq!(cloned.display_name(), "MiniMax");
    }

    #[test]
    fn test_settings_apply_masked_key_not_overwritten() {
        let mut provider = MiniMaxProvider {
            api_key: "original-key".to_string(),
            enabled: false,
            enabled_models: Vec::new(),
            custom_models: HashMap::new(),
        };
        let yaml: serde_yaml::Value = serde_yaml::from_str(r#"
            api_key: "***"
            enabled: true
        "#).unwrap();
        provider.provider_settings_apply(yaml).unwrap();
        assert_eq!(provider.api_key, "original-key");
        assert!(provider.enabled);
    }
}
