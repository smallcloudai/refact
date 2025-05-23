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

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpertInput {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpertPatch {
    pub owner_shared: Option<bool>,
    pub located_fgroup_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpertResponse {
    pub expert: Expert,
}

pub async fn get_expert(
    gcx: Arc<ARwLock<GlobalContext>>,
    expert_id: &str,
) -> Result<Expert, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;

    let query = r#"
    query GetExpert($id: String!) {
        expert(id: $id) {
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
        "id": expert_id
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
            if let Some(expert_value) = data.get("expert") {
                let expert: Expert = serde_json::from_value(expert_value.clone())
                    .map_err(|e| format!("Failed to parse expert: {}", e))?;

                info!("Successfully retrieved expert {}", expert_id);
                return Ok(expert);
            }
        }

        Err(format!(
            "Expert not found or unexpected response format: {}",
            response_body
        ))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get expert: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}
