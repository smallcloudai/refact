use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::caps::{
    BaseModelRecord, ChatModelRecord, CodeAssistantCaps, CompletionModelRecord, DefaultModels,
    EmbeddingModelRecord, default_chat_scratchpad, default_completion_scratchpad,
    default_completion_scratchpad_patch, default_embedding_batch, default_hf_tokenizer_template,
    default_rejection_threshold, relative_to_full_url
};

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

pub fn load_self_hosted_caps(
    self_hosted_caps: SelfHostedCaps,
    caps_url: &String,
    cmdline_api_key: &str,
) -> Result<CodeAssistantCaps, String> {
    let mut caps = CodeAssistantCaps {
        cloud_name: self_hosted_caps.cloud_name.clone(),

        telemetry_basic_dest: relative_to_full_url(caps_url, &self_hosted_caps.telemetry_endpoints.telemetry_basic_endpoint)?,
        telemetry_basic_retrieve_my_own: relative_to_full_url(caps_url, &self_hosted_caps.telemetry_endpoints.telemetry_basic_retrieve_my_own_endpoint)?,
        
        completion_models: IndexMap::new(),
        chat_models: IndexMap::new(),
        embedding_model: EmbeddingModelRecord::default(),

        defaults: DefaultModels { 
            completion_default_model: format!("{}/{}", self_hosted_caps.cloud_name, self_hosted_caps.completion.default_model), 
            chat_default_model: format!("{}/{}", self_hosted_caps.cloud_name, self_hosted_caps.chat.default_model),
            chat_thinking_model: String::new(),
            chat_light_model: String::new(),
        },
        customization: self_hosted_caps.customization,
        caps_version: self_hosted_caps.caps_version,

        hf_tokenizer_template: default_hf_tokenizer_template(),
    };

    let configure_base_model = |base_model: &mut BaseModelRecord, model_name: &str, endpoint: &str| -> Result<(), String> {
        base_model.name = model_name.to_string();
        base_model.id = format!("{}/{}", self_hosted_caps.cloud_name, model_name);
        if base_model.endpoint.is_empty() {
            base_model.endpoint = relative_to_full_url(caps_url, &endpoint.replace("$MODEL", model_name))?;
        }
        if let Some(tokenizer) = self_hosted_caps.tokenizer_endpoints.get(&base_model.name) {
            base_model.tokenizer = relative_to_full_url(caps_url, &tokenizer)?;
        }
        base_model.api_key = cmdline_api_key.to_string();
        base_model.endpoint_style = "openai".to_string();
        Ok(())
    };

    for (model_name, model_rec) in self_hosted_caps.completion.models {
        let mut base = BaseModelRecord{
            n_ctx: model_rec.n_ctx,
            ..Default::default()
        };
        configure_base_model(&mut base, &model_name, &self_hosted_caps.completion.endpoint)?;
        
        let (scratchpad, scratchpad_patch) = if !model_rec.supports_scratchpads.is_empty() {
            let scratchpad_name = model_rec.supports_scratchpads.keys().next().unwrap_or(&default_completion_scratchpad()).clone();
            let scratchpad_patch = model_rec.supports_scratchpads.values().next().unwrap_or(&serde_json::Value::Null).clone();
            (scratchpad_name, scratchpad_patch)
        } else {
            (default_completion_scratchpad(), default_completion_scratchpad_patch())
        };
        
        let completion_model = CompletionModelRecord {
            base,
            scratchpad,
            scratchpad_patch,
        };
        
        caps.completion_models.insert(completion_model.base.id.clone(), Arc::new(completion_model));
    }

    for (model_name, model_rec) in self_hosted_caps.chat.models {
        let mut base = BaseModelRecord{
            n_ctx: model_rec.n_ctx,
            ..Default::default()
        };
        configure_base_model(&mut base, &model_name, &self_hosted_caps.chat.endpoint)?;
        
        let (scratchpad, scratchpad_patch) = if !model_rec.supports_scratchpads.is_empty() {
            let scratchpad_name = model_rec.supports_scratchpads.keys().next().unwrap_or(&default_chat_scratchpad()).clone();
            let scratchpad_patch = model_rec.supports_scratchpads.values().next().unwrap_or(&serde_json::Value::Null).clone();
            (scratchpad_name, scratchpad_patch)
        } else {
            (default_chat_scratchpad(), serde_json::Value::Null)
        };
        
        let chat_model = ChatModelRecord {
            base,
            scratchpad,
            scratchpad_patch,
            supports_tools: model_rec.supports_tools,
            supports_multimodality: model_rec.supports_multimodality,
            supports_clicks: model_rec.supports_clicks,
            supports_agent: model_rec.supports_agent,
            supports_reasoning: model_rec.supports_reasoning,
            supports_boost_reasoning: model_rec.supports_boost_reasoning,
            default_temperature: model_rec.default_temperature,
        };
        
        caps.chat_models.insert(chat_model.base.id.clone(), Arc::new(chat_model));
    }

    if let Some(server_embedding_model) = self_hosted_caps.embedding.models
        .get(&self_hosted_caps.embedding.default_model).cloned() 
    {
        let mut embedding_model = EmbeddingModelRecord {
            base: BaseModelRecord { n_ctx: server_embedding_model.n_ctx, ..Default::default() },
            embedding_size: server_embedding_model.size,
            rejection_threshold: default_rejection_threshold(),
            embedding_batch: default_embedding_batch(),
        };
        configure_base_model(&mut embedding_model.base, &self_hosted_caps.embedding.default_model, &self_hosted_caps.embedding.endpoint)?;
        caps.embedding_model = embedding_model;
    }

    Ok(caps)
}
