use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_stop: bool,
    pub supports_scratchpads: HashMap<String, serde_json::Value>,
    pub default_scratchpad: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantRecommendations {
    pub cloud_name: String,
    pub endpoint_template: String,
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_completion_default_model: String,
    pub code_chat_models: HashMap<String, ModelRecord>,
    pub code_chat_default_model: String,
}

pub fn load_recommendations() -> Arc<StdRwLock<CodeAssistantRecommendations>> {
    let file_path = "code_assistant_recommendations.json";
    let mut file = File::open(file_path).expect(format!("Failed to open file '{}'", file_path).as_str());
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).expect(format!("Failed to read file '{}'", file_path).as_str());
    let r = serde_json::from_str(&buffer).expect("Failed to parse json");
    Arc::new(StdRwLock::new(r))
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
            "Model '{}' not found. This rust binary blob supports these models: {:?}",
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
    if user_wants_scratchpad!= "" {
        take_this_one = default_scratchpad;
    }
    if let Some(scratchpad_patch) = scratchpads.get(take_this_one) {
        return Ok((take_this_one.to_string(), scratchpad_patch));
    } else {
        return Err(format!(
            "Scratchpad '{}' not found. This rust binary blob supports these scratchpads: {:?}",
            take_this_one,
            scratchpads.keys()
        ));
    }
}
