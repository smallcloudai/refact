use std::path::PathBuf;
use std::sync::Arc;
use itertools::Itertools;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::{info, warn};


#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MemoRecord {
    pub iknow_id: String,
    pub iknow_tags: Vec<String>,
    pub iknow_memory: String,
}


pub async fn memories_migration(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_dir: PathBuf
) {
    // Disable migration for now
    if true {
        return;
    }  
    
    if let None = gcx.read().await.active_group_id.clone() {
        info!("No active group set up, skipping memory migration");
        return;
    }
    
    let legacy_db_path = config_dir.join("memories.sqlite");
    if !legacy_db_path.exists() {
        return;
    }
    
    info!("Found legacy memory database at {:?}, starting migration", legacy_db_path);
    
    let conn = match Connection::open(&legacy_db_path).await {
        Ok(conn) => conn,
        Err(e) => {
            warn!("Failed to open legacy database: {}", e);
            return;
        }
    };
    
    let memories: Vec<(String, String)> = match conn.call(|conn| {
        // Query all memories
        let mut stmt = conn.prepare("SELECT m_type, m_payload FROM memories")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })?;
        
        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        
        Ok(memories.into_iter().unique_by(|(_, m_payload)| m_payload.clone()).collect())
    }).await {
        Ok(memories) => memories,
        Err(e) => {
            warn!("Failed to query memories: {}", e);
            return;
        }
    };
    
    if memories.is_empty() {
        info!("No memories found in legacy database");
        return;
    }
    
    info!("Found {} memories in legacy database, migrating to cloud", memories.len());
    
    // Migrate each memory to the cloud
    let mut success_count = 0;
    let mut error_count = 0;
    for (m_type, m_payload) in memories {
        if m_payload.is_empty() {
            warn!("Memory payload is empty, skipping");
            continue;
        }
        match memories_add(gcx.clone(), &m_type, &m_payload, true).await {
            Ok(_) => {
                success_count += 1;
                if success_count % 10 == 0 {
                    info!("Migrated {} memories so far", success_count);
                }
            },
            Err(e) => {
                error_count += 1;
                warn!("Failed to migrate memory: {}", e);
            }
        }
    }
    
    info!("Memory migration complete: {} succeeded, {} failed", success_count, error_count);
    if success_count > 0 {
        match fs::remove_file(legacy_db_path.clone()).await {
            Ok(_) => info!("Removed legacy database: {:?}", legacy_db_path),
            Err(e) => warn!("Failed to remove legacy database: {}", e),
        }
    }
}

pub async fn memories_add(
    gcx: Arc<ARwLock<GlobalContext>>,
    m_type: &str,
    m_memory: &str,
    _unknown_project: bool
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    let query = r#"
        mutation CreateKnowledgeItem($input: FKnowledgeItemInput!) {
            knowledge_item_create(input: $input) {
                iknow_id
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
            "variables": { 
                "input": {
                    "iknow_memory": m_memory,
                    "located_fgroup_id": active_group_id,
                    "iknow_is_core": false,
                    "iknow_tags": vec![m_type.to_string()],
                    "owner_shared": false      
                }
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
            if let Some(_) = data.get("knowledge_item_create") {
                info!("Successfully added memory to remote server");
                return Ok(());
            }
        }
        Err("Failed to add memory to remote server".to_string())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to add memory to remote server: HTTP status {}, error: {}", status, error_text))
    }
}

pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    q: &String,
    top_n: usize,
) -> Result<Vec<MemoRecord>, String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    let query = r#"
        query KnowledgeSearch($fgroup_id: String!, $q: String!, $top_n: Int!) {
            knowledge_vecdb_search(fgroup_id: $fgroup_id, q: $q, top_n: $top_n) {
                iknow_id 
                iknow_memory
                iknow_tags
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
            "variables": {
                "fgroup_id": active_group_id,
                "q": q,
                "top_n": top_n,
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
            if let Some(memories_value) = data.get("knowledge_vecdb_search") {
                let memories: Vec<MemoRecord> = serde_json::from_value(memories_value.clone())
                    .map_err(|e| format!("Failed to parse expert: {}", e))?;
                return Ok(memories);
            }
        }
        Err("Failed to get memories from remote server".to_string())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to get memories from remote server: HTTP status {}, error: {}", status, error_text))
    }
}

pub async fn memories_get_core(
    gcx: Arc<ARwLock<GlobalContext>>
) -> Result<Vec<MemoRecord>, String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
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
    let response = client
        .post(&crate::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("User-Agent", "refact-lsp")
        .json(&json!({
            "query": query,
            "variables": {
              "fgroup_id": active_group_id
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
            if let Some(memories_value) = data.get("knowledge_get_cores") {
                let memories: Vec<MemoRecord> = serde_json::from_value(memories_value.clone())
                    .map_err(|e| format!("Failed to parse expert: {}", e))?;
                return Ok(memories);
            }
        }
        Err("Failed to get core memories from remote server".to_string())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to get core memories from remote server: HTTP status {}, error: {}", status, error_text))
    }
}
