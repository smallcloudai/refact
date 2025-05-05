use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::json;

use crate::caps::{
    BaseModelRecord, ChatModelRecord, CodeAssistantCaps, CompletionModelRecord, DefaultModels,
    EmbeddingModelRecord, default_chat_scratchpad, default_completion_scratchpad,
    default_completion_scratchpad_patch, default_embedding_batch, default_hf_tokenizer_template,
    default_rejection_threshold, relative_to_full_url, normalize_string, resolve_relative_urls
};
use crate::caps::providers;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsModelRecord {
    pub n_ctx: usize,

    #[serde(default)]
    pub supports_scratchpads: HashMap<String, serde_json::Value>,

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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsEmbeddingModelRecord {
    pub n_ctx: usize,
    pub size: i32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsCompletion {
    pub endpoint: String,
    pub models: IndexMap<String, SelfHostedCapsModelRecord>,
    pub default_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsChat {
    pub endpoint: String,
    pub models: IndexMap<String, SelfHostedCapsModelRecord>,
    pub default_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsEmbedding {
    pub endpoint: String,
    pub models: IndexMap<String, SelfHostedCapsEmbeddingModelRecord>,
    pub default_model: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCapsTelemetryEndpoints {
    pub telemetry_basic_endpoint: String,
    pub telemetry_basic_retrieve_my_own_endpoint: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SelfHostedCaps {
    #[serde(deserialize_with = "normalize_string")]
    pub cloud_name: String,

    pub completion: SelfHostedCapsCompletion,
    pub chat: SelfHostedCapsChat,
    pub embedding: SelfHostedCapsEmbedding,

    pub telemetry_endpoints: SelfHostedCapsTelemetryEndpoints,
    pub tokenizer_endpoints: HashMap<String, String>,

    #[serde(default)]
    pub customization: String,
    pub caps_version: i64,
}

fn configure_base_model(
    base_model: &mut BaseModelRecord,
    model_name: &str,
    endpoint: &str,
    cloud_name: &str,
    tokenizer_endpoints: &HashMap<String, String>,
    caps_url: &String,
    cmdline_api_key: &str,
) -> Result<(), String> {
    base_model.name = model_name.to_string();
    base_model.id = format!("{}/{}", cloud_name, model_name);
    if base_model.endpoint.is_empty() {
        base_model.endpoint = relative_to_full_url(caps_url, &endpoint.replace("$MODEL", model_name))?;
    }
    if let Some(tokenizer) = tokenizer_endpoints.get(&base_model.name) {
        base_model.tokenizer = relative_to_full_url(caps_url, &tokenizer)?;
    }
    base_model.api_key = cmdline_api_key.to_string();
    base_model.endpoint_style = "openai".to_string();
    Ok(())
}

impl SelfHostedCapsModelRecord {
    fn get_completion_scratchpad(&self) -> (String, serde_json::Value) {
        if !self.supports_scratchpads.is_empty() {
            let scratchpad_name = self.supports_scratchpads.keys().next().unwrap_or(&default_completion_scratchpad()).clone();
            let scratchpad_patch = self.supports_scratchpads.values().next().unwrap_or(&serde_json::Value::Null).clone();
            (scratchpad_name, scratchpad_patch)
        } else {
            (default_completion_scratchpad(), default_completion_scratchpad_patch())
        }
    }

    fn get_chat_scratchpad(&self) -> (String, serde_json::Value) {
        if !self.supports_scratchpads.is_empty() {
            let scratchpad_name = self.supports_scratchpads.keys().next().unwrap_or(&default_chat_scratchpad()).clone();
            let scratchpad_patch = self.supports_scratchpads.values().next().unwrap_or(&serde_json::Value::Null).clone();
            (scratchpad_name, scratchpad_patch)
        } else {
            (default_chat_scratchpad(), serde_json::Value::Null)
        }
    }

    pub fn into_completion_model(
        &self,
        model_name: &str,
        self_hosted_caps: &SelfHostedCaps,
        caps_url: &String,
        cmdline_api_key: &str,
    ) -> Result<CompletionModelRecord, String> {
        let mut base = BaseModelRecord {
            n_ctx: self.n_ctx,
            enabled: true,
            ..Default::default()
        };

        configure_base_model(
            &mut base,
            model_name,
            &self_hosted_caps.completion.endpoint,
            &self_hosted_caps.cloud_name,
            &self_hosted_caps.tokenizer_endpoints,
            caps_url,
            cmdline_api_key,
        )?;

        let (scratchpad, scratchpad_patch) = self.get_completion_scratchpad();

        Ok(CompletionModelRecord {
            base,
            scratchpad,
            scratchpad_patch,
            model_family: None,
        })
    }
}

impl SelfHostedCapsModelRecord {
    pub fn into_chat_model(
        &self,
        model_name: &str,
        self_hosted_caps: &SelfHostedCaps,
        caps_url: &String,
        cmdline_api_key: &str,
    ) -> Result<ChatModelRecord, String> {
        let mut base = BaseModelRecord {
            n_ctx: self.n_ctx,
            enabled: true,
            ..Default::default()
        };

        let (scratchpad, scratchpad_patch) = self.get_chat_scratchpad();

        // Non passthrough models, don't support endpoints of `/v1/chat/completions` in openai style, only `/v1/completions`
        let endpoint_to_use = if scratchpad == "PASSTHROUGH" {
            &self_hosted_caps.chat.endpoint
        } else {
            &self_hosted_caps.completion.endpoint
        };

        configure_base_model(
            &mut base,
            model_name,
            endpoint_to_use,
            &self_hosted_caps.cloud_name,
            &self_hosted_caps.tokenizer_endpoints,
            caps_url,
            cmdline_api_key,
        )?;

        Ok(ChatModelRecord {
            base,
            scratchpad,
            scratchpad_patch,
            supports_tools: self.supports_tools,
            supports_multimodality: self.supports_multimodality,
            supports_clicks: self.supports_clicks,
            supports_agent: self.supports_agent,
            supports_reasoning: self.supports_reasoning.clone(),
            supports_boost_reasoning: self.supports_boost_reasoning,
            default_temperature: self.default_temperature,
        })
    }
}

impl SelfHostedCapsEmbeddingModelRecord {
    pub fn into_embedding_model(
        &self,
        model_name: &str,
        self_hosted_caps: &SelfHostedCaps,
        caps_url: &String,
        cmdline_api_key: &str,
    ) -> Result<EmbeddingModelRecord, String> {
        let mut embedding_model = EmbeddingModelRecord {
            base: BaseModelRecord { n_ctx: self.n_ctx, enabled: true, ..Default::default() },
            embedding_size: self.size,
            rejection_threshold: default_rejection_threshold(),
            embedding_batch: default_embedding_batch(),
        };

        configure_base_model(
            &mut embedding_model.base,
            model_name,
            &self_hosted_caps.embedding.endpoint,
            &self_hosted_caps.cloud_name,
            &self_hosted_caps.tokenizer_endpoints,
            caps_url,
            cmdline_api_key,
        )?;

        Ok(embedding_model)
    }
}


impl SelfHostedCaps {
    pub fn into_caps(self, caps_url: &String, cmdline_api_key: &str) -> Result<CodeAssistantCaps, String> {
        let mut caps = CodeAssistantCaps {
            cloud_name: self.cloud_name.clone(),

            telemetry_basic_dest: relative_to_full_url(caps_url, &self.telemetry_endpoints.telemetry_basic_endpoint)?,
            telemetry_basic_retrieve_my_own: relative_to_full_url(caps_url, &self.telemetry_endpoints.telemetry_basic_retrieve_my_own_endpoint)?,

            completion_models: IndexMap::new(),
            chat_models: IndexMap::new(),
            embedding_model: EmbeddingModelRecord::default(),

            defaults: DefaultModels {
                completion_default_model: format!("{}/{}", self.cloud_name, self.completion.default_model),
                chat_default_model: format!("{}/{}", self.cloud_name, self.chat.default_model),
                chat_thinking_model: String::new(),
                chat_light_model: format!("{}/{}", self.cloud_name, self.chat.default_model),
            },
            customization: self.customization.clone(),
            caps_version: self.caps_version,

            hf_tokenizer_template: default_hf_tokenizer_template(),

            metadata: json!({}),
        };

        for (model_name, model_rec) in &self.completion.models {
            let completion_model = model_rec.into_completion_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;

            caps.completion_models.insert(completion_model.base.id.clone(), Arc::new(completion_model));
        }

        for (model_name, model_rec) in &self.chat.models {
            let chat_model = model_rec.into_chat_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;

            caps.chat_models.insert(chat_model.base.id.clone(), Arc::new(chat_model));
        }

        if let Some((model_name, model_rec)) = self.embedding.models.get_key_value(&self.embedding.default_model) {
            let embedding_model = model_rec.into_embedding_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;
            caps.embedding_model = embedding_model;
        }

        Ok(caps)
    }

    pub fn into_provider(self, caps_url: &String, cmdline_api_key: &str) -> Result<providers::CapsProvider, String> {
        let mut provider = providers::CapsProvider {
            name: self.cloud_name.clone(),
            enabled: true,
            supports_completion: true,
            endpoint_style: "openai".to_string(),
            completion_endpoint: self.completion.endpoint.clone(),
            chat_endpoint: self.chat.endpoint.clone(),
            embedding_endpoint: self.embedding.endpoint.clone(),
            api_key: cmdline_api_key.to_string(),
            tokenizer_api_key: cmdline_api_key.to_string(),
            code_completion_n_ctx: 0,
            support_metadata: false,
            completion_models: IndexMap::new(),
            chat_models: IndexMap::new(),
            embedding_model: EmbeddingModelRecord::default(),
            models_dict_patch: IndexMap::new(),
            defaults: DefaultModels {
                completion_default_model: self.completion.default_model.clone(),
                chat_default_model: self.chat.default_model.clone(),
                chat_thinking_model: String::new(),
                chat_light_model: String::new(),
            },
            running_models: Vec::new(),
        };

        for (model_name, model_rec) in &self.completion.models {
            let completion_model = model_rec.into_completion_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;

            provider.completion_models.insert(model_name.clone(), completion_model);
        }

        for (model_name, model_rec) in &self.chat.models {
            let chat_model = model_rec.into_chat_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;

            provider.chat_models.insert(model_name.clone(), chat_model);
        }

        if let Some((model_name, model_rec)) = self.embedding.models.get_key_value(&self.embedding.default_model) {
            let embedding_model = model_rec.into_embedding_model(
                model_name,
                &self,
                caps_url,
                cmdline_api_key,
            )?;
            provider.embedding_model = embedding_model;
        }

        resolve_relative_urls(&mut provider, caps_url)?;

        Ok(provider)
    }
}
