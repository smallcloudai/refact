use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

#[derive(Debug, Serialize, Deserialize)]
pub struct Thread {
    pub owner_fuser_id: String,
    pub owner_shared: bool,
    pub located_fgroup_id: String,
    pub ft_id: String,
    pub ft_fexp_name: String,
    pub ft_title: String,
    pub ft_toolset: Vec<Value>,
    pub ft_belongs_to_fce_id: Option<String>,
    pub ft_model: String,
    pub ft_temperature: f64,
    pub ft_max_new_tokens: i32,
    pub ft_n: i32,
    pub ft_error: Option<String>,
    pub ft_need_assistant: i32,
    pub ft_need_tool_calls: i32,
    pub ft_anything_new: bool,
    pub ft_created_ts: f64,
    pub ft_updated_ts: f64,
    pub ft_archived_ts: f64,
    pub ft_locked_by: String,
}

pub async fn get_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
) -> Result<Thread, String> {
    let client = Client::new();
    let (api_key, located_fgroup_id) = {
        let gcx_read = gcx.read().await;
        (gcx_read.cmdline.api_key.clone(),
         gcx_read.active_group_id.clone().unwrap_or_default())
    };
    let query = r#"
    query GetAllThreads($group_id: String!) {
        thread_list(located_fgroup_id: $group_id) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_fexp_name,
            ft_title
            ft_belongs_to_fce_id
            ft_model
            ft_temperature
            ft_max_new_tokens
            ft_n
            ft_error
            ft_toolset
            ft_need_assistant
            ft_need_tool_calls
            ft_anything_new
            ft_created_ts
            ft_updated_ts
            ft_archived_ts
            ft_locked_by
        }
    }
    "#;
    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": {"group_id": located_fgroup_id}
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
            if let Some(threads) = data.get("thread_list") {
                if let Some(threads_array) = threads.as_array() {
                    for thread_value in threads_array {
                        if thread_value["ft_id"] == thread_id {
                            let thread: Thread = serde_json::from_value(thread_value.clone())
                                .map_err(|e| format!("Failed to parse thread: {}", e))?;
                            return Ok(thread);
                        }
                    }
                    return Err(format!("Thread with ID {} not found", thread_id));
                }
            }
        }
        Err(format!(
            "Thread not found or unexpected response format: {}",
            response_body
        ))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get thread: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}

pub async fn set_thread_toolset(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    ft_toolset: Vec<Value>,
) -> Result<Vec<Value>, String> {
    let client = Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let mutation = r#"
    mutation UpdateThread($thread_id: String!, $patch: FThreadPatch!) {
        thread_patch(id: $thread_id, patch: $patch) {
            ft_toolset
        }
    }
    "#;
    let variables = json!({
        "thread_id": thread_id,
        "patch": {
            "ft_toolset": serde_json::to_string(&ft_toolset).unwrap()
        }
    });
    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": mutation,
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
            if let Some(ft_toolset_json) = data.get("thread_patch") {
                let ft_toolset: Vec<Value> =
                    serde_json::from_value(ft_toolset_json["ft_toolset"].clone())
                        .map_err(|e| format!("Failed to parse updated thread: {}", e))?;
                return Ok(ft_toolset);
            }
        }
        Err(format!("Unexpected response format: {}", response_body))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to update thread: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}

pub async fn lock_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    hash: &str,
) -> Result<(), String> {
    let client = Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let worker_name = format!("refact-lsp:{hash}");
    let query = r#"
        mutation AdvanceLock($ft_id: String!, $worker_name: String!) {
            thread_lock(ft_id: $ft_id, worker_name: $worker_name)
        } 
    "#;
    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": {"ft_id": thread_id, "worker_name": worker_name}
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
            if data.get("thread_lock").is_some() {
                return Ok(());
            } else {
                return Err(format!("Thread {thread_id} is locked by another worker"));
            }
        }
        Err(format!(
            "Thread not found or unexpected response format: {}",
            response_body
        ))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get thread: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}

pub async fn unlock_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: String,
    hash: String,
) -> Result<(), String> {
    let client = Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let worker_name = format!("refact-lsp:{hash}");
    let query = r#"
        mutation AdvanceUnlock($ft_id: String!, $worker_name: String!) {
            thread_unlock(ft_id: $ft_id, worker_name: $worker_name)
        }
    "#;
    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": {"ft_id": thread_id, "worker_name": worker_name}
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
            if data.get("thread_unlock").is_some() {
                return Ok(());
            } else {
                return Err(format!("Cannot unlock thread {thread_id}"));
            }
        }
        Err(format!(
            "Thread not found or unexpected response format: {}",
            response_body
        ))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get thread: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}
