use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct Expert {
    pub owner_fuser_id: Option<String>,
    pub owner_shared: bool,
    pub located_fgroup_id: Option<String>,
    pub fexp_id: String,
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
    cmd_address_url: &str,
    api_key: &str,
    fexp_id: &str
) -> Result<Expert, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};
    
    let query = r#"
    query GetExpert($id: String!) {
        expert_get(id: $id) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            fexp_id
            fexp_name
            fexp_system_prompt
            fexp_python_kernel
            fexp_block_tools
            fexp_allow_tools
        }
    }
    "#;
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    info!("get_expert: address={}, fexp_id={}", config.address, fexp_id);
    execute_graphql::<Expert, _>(
        config,
        query,
        json!({"id": fexp_id}),
        "expert_get"
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn expert_choice_consequences(
    cmd_address_url: &str,
    api_key: &str,
    fexp_id: &str,
    fgroup_id: &str,
) -> Result<String, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};
    
    #[derive(Deserialize, Debug)]
    struct ModelInfo {
        provm_name: String,
    }
    
    let query = r#"
    query GetExpertModel($fexp_id: String!, $inside_fgroup_id: String!) {
        expert_choice_consequences(fexp_id: $fexp_id, inside_fgroup_id: $inside_fgroup_id) {
            models {
                provm_name
            }
        }
    }
    "#;
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };
    
    let variables = json!({
        "fexp_id": fexp_id,
        "inside_fgroup_id": fgroup_id
    });

    info!("expert_choice_consequences: address={}, fexp_id={}, inside_fgroup_id={}", config.address, fexp_id, fgroup_id);
    #[derive(Deserialize, Debug)]
    struct Consequences {
        models: Vec<ModelInfo>,
    }
    let result: Consequences = execute_graphql(
        config,
        query,
        variables,
        "expert_choice_consequences"
    )
    .await
    .map_err(|e| e.to_string())?;
    
    if result.models.is_empty() {
        return Err(format!("No models found for the expert with name {}", fexp_id));
    }
    
    Ok(result.models[0].provm_name.clone())
}
