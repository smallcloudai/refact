use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

#[derive(Debug, Serialize, Deserialize)]
pub struct Expert {
    pub owner_fuser_id: String,
    pub owner_shared: bool,
    pub located_fgroup_id: String,
    pub fexp_name: String,
    pub fexp_ver_major: i32,
    pub fexp_ver_minor: i32,
    pub fexp_system_prompt: String,
    pub fexp_python_kernel: String,
    pub fexp_block_tools: String,
    pub fexp_allow_tools: String,
}

impl Expert {
    pub fn get_blocked_tools(&self) -> Result<Vec<String>, String> {
        serde_json::from_str(&self.fexp_block_tools)
            .map_err(|e| format!("Failed to decode block tools: {}", e))
    }

    pub fn get_allowed_tools(&self) -> Result<Vec<String>, String> {
        serde_json::from_str(&self.fexp_allow_tools)
            .map_err(|e| format!("Failed to decode allow tools: {}", e))
    }
}

pub async fn get_expert(
    gcx: Arc<ARwLock<GlobalContext>>,
    expert_name: &str,
) -> Result<Expert, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;

    let query = r#"
    query GetExpert($located_fgroup_id: String!, $skip: Int!, $limit: Int!) {
        expert_list(located_fgroup_id: $located_fgroup_id, skip: $skip, limit: $limit) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            fexp_name
            fexp_ver_major
            fexp_ver_minor
            fexp_system_prompt
            fexp_python_kernel
            fexp_block_tools
            fexp_allow_tools
        }
    }
    "#;

    let variables = json!({
        "located_fgroup_id": "solar_root".to_string(),
        "skip": 0,
        "limit": 100
    });

    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": variables
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send GraphQL request: {}", e))?;

    if response.status().is_success() {
        let response_body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let response_json: Value = serde_json::from_str(&response_body)
            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }

        if let Some(data) = response_json.get("data") {
            if let Some(expert_list) = data.get("expert_list") {
                if let Some(experts) = expert_list.as_array() {
                    // Find the expert with the matching name
                    for expert_value in experts {
                        if let Some(name) = expert_value.get("fexp_name") {
                            if name.as_str() == Some(expert_name) {
                                let expert: Expert = serde_json::from_value(expert_value.clone())
                                    .map_err(|e| format!("Failed to parse expert: {}", e))?;

                                info!("Successfully retrieved expert {}", expert_name);
                                return Ok(expert);
                            }
                        }
                    }
                }
            }
        }

        Err(format!(
            "Expert with name '{}' not found or unexpected response format: {}",
            expert_name, response_body
        ))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get expert with name {}: HTTP status {}, error: {}",
            expert_name, status, error_text
        ))
    }
}
