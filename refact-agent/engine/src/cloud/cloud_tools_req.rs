use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

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
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str,
) -> Result<Vec<CloudTool>, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};
    
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
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    info!("get_cloud_tools: address={}, located_fgroup_id={}", config.address, located_fgroup_id);
    execute_graphql::<Vec<CloudTool>, _>(
        config,
        query,
        json!({"located_fgroup_id": located_fgroup_id}),
        "cloud_tools_list"
    )
    .await
    .map_err(|e| e.to_string())
}
