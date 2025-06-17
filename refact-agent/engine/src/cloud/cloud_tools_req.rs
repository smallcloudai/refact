use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudTool {
    pub owner_fuser_id: Option<String>,
    pub located_fgroup_id: Option<String>,
    pub ctool_id: String,
    pub ctool_name: String,
    pub ctool_description: String,
    pub ctool_confirmed_exists_ts: Option<f32>,
    pub ctool_parameters: Value,
}

impl CloudTool {
    pub fn into_openai_style(self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.ctool_name,
                "description": self.ctool_description,
                "parameters": self.ctool_parameters,
            }
        })
    }
}

pub async fn get_cloud_tools(
    api_key: String,
    located_fgroup_id: &str,
) -> Result<Vec<CloudTool>, String> {
    let client = Client::new();
    let query = r#"
    query GetCloudTools($located_fgroup_id: String!) {
        cloud_tools_list(located_fgroup_id: $located_fgroup_id, include_offline: true) {
            owner_fuser_id
            located_fgroup_id
            ctool_id
            ctool_name
            ctool_description
            ctool_confirmed_exists_ts
            ctool_parameters
        }
    }
    "#;
    let response = client
        .post(&crate::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("User-Agent", "refact-lsp")
        .json(&json!({
            "query": query,
            "variables": {"located_fgroup_id": located_fgroup_id}
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
            if let Some(tools) = data.get("cloud_tools_list") {
                let cloud_tools: Vec<CloudTool> = serde_json::from_value(tools.clone())
                    .map_err(|e| format!("Failed to parse expert: {}", e))?;
                return Ok(cloud_tools);
            }
        }
        Err("Failed to get cloud tools: no data in response".to_string())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to get cloud tools: status: {}, error: {}", status, error_text))
    }
}
