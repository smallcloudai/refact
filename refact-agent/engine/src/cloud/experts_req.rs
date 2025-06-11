use log::error;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Expert {
    pub owner_fuser_id: Option<String>,
    pub owner_shared: bool,
    pub located_fgroup_id: Option<String>,
    pub fexp_name: String,
    pub fexp_system_prompt: String,
    pub fexp_python_kernel: String,
    pub fexp_block_tools: String,
    pub fexp_allow_tools: String,
}

impl Expert {
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        let mut blocked = false;
        if !self.fexp_block_tools.trim().is_empty() {
            match Regex::new(&self.fexp_block_tools) {
                Ok(re) => {
                    if re.is_match(tool_name) {
                        blocked = true;
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to compile fexp_block_tools regex: {}: {}",
                        self.fexp_block_tools, e
                    );
                }
            }
        }
        // Allow if matches allow regex, even if blocked
        if !self.fexp_allow_tools.trim().is_empty() {
            match Regex::new(&self.fexp_allow_tools) {
                Ok(re) => {
                    if re.is_match(tool_name) {
                        return true;
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to compile fexp_allow_tools regex: {}: {}",
                        self.fexp_allow_tools, e
                    );
                }
            }
        }

        !blocked
    }
}

pub async fn get_expert(
    api_key: String,
    expert_name: &str
) -> Result<Expert, String> {
    let client = Client::new();
    let query = r#"
    query GetExpert($id: String!) {
        expert_get(id: $id) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            fexp_name
            fexp_system_prompt
            fexp_python_kernel
            fexp_block_tools
            fexp_allow_tools
        }
    }
    "#;
    let response = client
        .post(&crate::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": { 
                "id": expert_name
            }
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
            if let Some(expert_value) = data.get("expert_get") {
                let expert: Expert = serde_json::from_value(expert_value.clone())
                    .map_err(|e| format!("Failed to parse expert: {}", e))?;
                return Ok(expert);
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
