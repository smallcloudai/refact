use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelRecord {
    pub n_ctx: usize,
    #[serde(default)]
    pub supports_stop: bool,
    pub supports_scratchpads: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeAssistantRecommendations {
    pub cloud_name: String,
    pub code_completion_models: HashMap<String, ModelRecord>,
    pub code_chat_models: HashMap<String, ModelRecord>,
}

pub fn load_recommendations() -> Arc<ARwLock<CodeAssistantRecommendations>> {
    let file_path = "code_assistant_recommendations.json";
    let mut file = File::open(file_path).expect(format!("Failed to open file '{}'", file_path).as_str());
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).expect(format!("Failed to read file '{}'", file_path).as_str());
    let r = serde_json::from_str(&buffer).expect("Failed to parse json");
    Arc::new(ARwLock::new(r))
}

