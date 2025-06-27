use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock as ARwLock;
use tracing::info;
use crate::global_context::GlobalContext;
use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};

#[derive(Serialize, Deserialize, Debug)]
pub struct MemoRecord {
    pub iknow_id: String,
    pub iknow_tags: Vec<String>,
    pub iknow_memory: String,
}

pub async fn memories_add(
    gcx: Arc<ARwLock<GlobalContext>>,
    m_type: &str,
    m_memory: &str,
) -> Result<(), String> {
    let (cmd_address_url, api_key) = {
        let gcx_read = gcx.read().await;
        (gcx_read.cmdline.address_url.clone(), gcx_read.cmdline.api_key.clone())
    };
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    
    let query = r#"
        mutation CreateKnowledgeItem($input: FKnowledgeItemInput!) {
            knowledge_item_create(input: $input) {
                iknow_id
            }
        }
    "#;
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key,
        ..Default::default()
    };
    
    let variables = json!({
        "input": {
            "iknow_memory": m_memory,
            "located_fgroup_id": active_group_id,
            "iknow_is_core": false,
            "iknow_tags": vec![m_type.to_string()],
            "owner_shared": false      
        }
    });

    info!("memories_add: address={}, iknow_memory={}, located_fgroup_id={}, iknow_is_core={}, iknow_tags={}, owner_shared={}",
        config.address, m_memory, active_group_id, false, m_type, false
    );
    execute_graphql::<serde_json::Value, _>(
        config,
        query,
        variables,
        "knowledge_item_create"
    )
    .await
    .map_err(|e| e.to_string())?;
    info!("Successfully added memory to remote server");
    
    Ok(())
}

pub async fn memories_get_core(
    gcx: Arc<ARwLock<GlobalContext>>
) -> Result<Vec<MemoRecord>, String> {
    let (cmd_address_url, api_key) = {
        let gcx_read = gcx.read().await;
        (gcx_read.cmdline.address_url.clone(), gcx_read.cmdline.api_key.clone())
    };
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    
    let query = r#"
        query KnowledgeSearch($fgroup_id: String!) {
            knowledge_get_cores(fgroup_id: $fgroup_id) {
                iknow_id 
                iknow_memory
                iknow_tags
            }
        }
    "#;
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url,
        api_key,
        ..Default::default()
    };
    
    let variables = json!({
        "fgroup_id": active_group_id
    });
    info!("memories_get_core: address={}, fgroup_id={}", config.address, active_group_id);
    let memories: Vec<MemoRecord> = execute_graphql(
        config,
        query,
        variables,
        "knowledge_get_cores"
    )
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(memories)
}
