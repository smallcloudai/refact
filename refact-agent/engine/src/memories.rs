use std::path::PathBuf;
use std::sync::Arc;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::{info, warn};


#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MemoRecord {
    pub iknow_id: i64,
    pub iknow_type: String,
    pub iknow_origin: String,
    pub iknow_memory: String,
    pub iknow_times_used: i64,
    pub iknow_mstat_relevant: i64,
}


pub async fn memories_migration(
    gcx: Arc<ARwLock<GlobalContext>>,
    config_dir: PathBuf
) {
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
    
    let memories: Vec<(String, String, String)> = match conn.call(|conn| {
        // Query all memories
        let mut stmt = conn.prepare("SELECT m_type, m_payload, m_origin FROM memories")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        
        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        
        Ok(memories.into_iter().unique_by(|(_, m_payload, _)| m_payload.clone()).collect())
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
    for (m_type,  m_payload, m_origin) in memories {
        if m_payload.is_empty() {
            warn!("Memory payload is empty, skipping");
            continue;
        }
        match memories_add(gcx.clone(), &m_type, &m_payload, Some(m_origin), true).await {
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
    m_origin: Option<String>,
    unknown_project: bool
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    let mut body = serde_json::json!({
        "knowledge_type": m_type,
        "knowledge_origin": m_origin.unwrap_or_else(|| "user-created".to_string()),
        "knowledge_memory": m_memory
    });
    if !unknown_project {
        body["group_id"] = Value::from(active_group_id);
    }
    let response = client.post(
        format!("https://test-teams-v1.smallcloud.ai/v1/knowledge/upload?group_id={}", active_group_id)
    )
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Successfully added memory to remote server");
                Ok(())
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to add memory: HTTP status {}, error: {}", status, error_text))
            }
        },
        Err(e) => Err(format!("Failed to send memory add request: {}", e))
    }
}


pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &String,
    top_n: usize,
) -> Result<Vec<MemoRecord>, String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    
    let url = format!("https://test-teams-v1.smallcloud.ai/v1/vecdb-search?limit={}", top_n);
    let body = serde_json::json!({
        "group_id": active_group_id,
        "q": query
    });
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_body = resp.text().await.map_err(|e| format!("Failed to read response body: {}", e))?;
                let results: Vec<MemoRecord> = serde_json::from_str(&response_body)
                    .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
                Ok(results)
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to search memories: HTTP status {}, error: {}", status, error_text))
            }
        },
        Err(e) => Err(format!("Failed to send memory search request: {}", e))
    }
}

pub async fn memories_get_core(
    gcx: Arc<ARwLock<GlobalContext>>
) -> Result<Vec<MemoRecord>, String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let active_group_id = gcx.read().await.active_group_id.clone()
        .ok_or("active_group_id must be set")?;
    let url = "https://test-teams-v1.smallcloud.ai/v1/knowledge/get_cores".to_string();
    let body = serde_json::json!({"group_id": active_group_id});
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_body = resp.text().await.map_err(|e| format!("Failed to read response body: {}", e))?;
                let results: Vec<MemoRecord> = serde_json::from_str(&response_body)
                    .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
                Ok(results)
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to search memories: HTTP status {}, error: {}", status, error_text))
            }
        },
        Err(e) => Err(format!("Failed to send memory search request: {}", e))
    }
}
