use log::{error, info};
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
    pub ft_title: String,
    pub ft_belongs_to_fce_id: Option<String>,
    pub ft_model: String,
    pub ft_temperature: f64,
    pub ft_max_new_tokens: i32,
    pub ft_n: i32,
    pub ft_error: String,
    pub ft_need_assistant: i32,
    pub ft_need_tool_calls: i32,
    pub ft_anything_new: bool,
    pub ft_created_ts: f64,
    pub ft_updated_ts: f64,
    pub ft_archived_ts: f64,
    pub ft_locked_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadCreateInput {
    pub owner_shared: bool,
    pub located_fgroup_id: String,
    pub ft_title: String,
    pub ft_belongs_to_fce_id: Option<String>,
    pub ft_model: String,
    pub ft_temperature: f64,
    pub ft_max_new_tokens: i32,
    pub ft_n: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadPatchInput {
    pub owner_shared: Option<bool>,
    pub located_fgroup_id: Option<String>,
    pub ft_title: Option<String>,
    pub ft_belongs_to_fce_id: Option<String>,
    pub ft_model: Option<String>,
    pub ft_temperature: Option<f64>,
    pub ft_max_new_tokens: Option<i32>,
    pub ft_n: Option<i32>,
    pub ft_error: Option<String>,
    pub ft_anything_new: Option<bool>,
    pub ft_archived_ts: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadsResponse {
    pub threads: Vec<Thread>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadResponse {
    pub thread: Thread,
}

/// ThreadMessage represents a message in a thread
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadMessage {
    pub ftm_belongs_to_ft_id: String,
    pub ftm_alt: i32,
    pub ftm_num: i32,
    pub ftm_prev_alt: i32,
    pub ftm_role: String,
    pub ftm_content: Value,
    pub ftm_tool_calls: Option<Value>,
    pub ftm_call_id: String,
    pub ftm_usage: Option<Value>,
    pub ftm_created_ts: f64,
}

/// ThreadMessagesResponse represents the response from the server when fetching thread messages
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadMessagesResponse {
    pub messages: Vec<ThreadMessage>,
}

pub async fn get_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
) -> Result<Thread, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;

    // First, we need to get the group ID for this thread
    // We'll use a workaround by querying all threads and filtering client-side
    // This is not efficient but will work until a better API is available
    let query = r#"
    query GetAllThreads($group_id: String!, $limit: Int!) {
        thread_list(
            located_fgroup_id: $group_id, 
            skip: 0, 
            limit: $limit,
            sort_by: ""
        ) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_title
            ft_belongs_to_fce_id
            ft_model
            ft_temperature
            ft_max_new_tokens
            ft_n
            ft_error
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

    let variables = json!({
        "group_id": crate::cloud::constants::DEFAULT_FGROUP_ID,
        "limit": crate::cloud::constants::DEFAULT_LIMIT
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

        // Check for GraphQL errors
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }

        // Extract the thread
        if let Some(data) = response_json.get("data") {
            if let Some(threads) = data.get("thread_list") {
                if let Some(threads_array) = threads.as_array() {
                    // Find the thread with the matching ID
                    for thread_value in threads_array {
                        if let Some(id) = thread_value.get("ft_id") {
                            if let Some(id_str) = id.as_str() {
                                if id_str == thread_id {
                                    let thread: Thread = serde_json::from_value(
                                        thread_value.clone(),
                                    )
                                    .map_err(|e| format!("Failed to parse thread: {}", e))?;

                                    info!("Successfully retrieved thread {}", thread_id);
                                    return Ok(thread);
                                }
                            }
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

pub async fn update_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    patch: ThreadPatchInput,
) -> Result<Thread, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;

    // Construct the GraphQL mutation
    let mutation = r#"
    mutation UpdateThread($thread_id: String!, $patch: FThreadPatch!) {
        thread_update(id: $thread_id, patch: $patch) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_title
            ft_belongs_to_fce_id
            ft_model
            ft_temperature
            ft_max_new_tokens
            ft_n
            ft_error
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

    // Create a patch object with only the fields that are set
    let mut patch_obj = json!({});

    if let Some(owner_shared) = patch.owner_shared {
        patch_obj["owner_shared"] = json!(owner_shared);
    }
    if let Some(located_fgroup_id) = &patch.located_fgroup_id {
        patch_obj["located_fgroup_id"] = json!(located_fgroup_id);
    }
    if let Some(ft_title) = &patch.ft_title {
        patch_obj["ft_title"] = json!(ft_title);
    }
    if let Some(ft_belongs_to_fce_id) = &patch.ft_belongs_to_fce_id {
        patch_obj["ft_belongs_to_fce_id"] = json!(ft_belongs_to_fce_id);
    }
    if let Some(ft_model) = &patch.ft_model {
        patch_obj["ft_model"] = json!(ft_model);
    }
    if let Some(ft_temperature) = patch.ft_temperature {
        patch_obj["ft_temperature"] = json!(ft_temperature);
    }
    if let Some(ft_max_new_tokens) = patch.ft_max_new_tokens {
        patch_obj["ft_max_new_tokens"] = json!(ft_max_new_tokens);
    }
    if let Some(ft_n) = patch.ft_n {
        patch_obj["ft_n"] = json!(ft_n);
    }
    if let Some(ft_error) = &patch.ft_error {
        patch_obj["ft_error"] = json!(ft_error);
    }
    if let Some(ft_anything_new) = patch.ft_anything_new {
        patch_obj["ft_anything_new"] = json!(ft_anything_new);
    }
    if let Some(ft_archived_ts) = patch.ft_archived_ts {
        patch_obj["ft_archived_ts"] = json!(ft_archived_ts);
    }

    let variables = json!({
        "thread_id": thread_id,
        "patch": patch_obj
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

        // Check for GraphQL errors
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }

        // Extract the updated thread
        if let Some(data) = response_json.get("data") {
            if let Some(thread) = data.get("thread_update") {
                let thread: Thread = serde_json::from_value(thread.clone())
                    .map_err(|e| format!("Failed to parse updated thread: {}", e))?;

                info!("Successfully updated thread {}", thread_id);
                return Ok(thread);
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

pub async fn get_thread_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    alt: i32,
) -> Result<Vec<ThreadMessage>, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;

    // Construct the GraphQL query
    let query = r#"
    query GetThreadMessagesByAlt($thread_id: String!, $alt: Int!) {
        thread_messages_by_alt(
            ft_id: $thread_id,
            alt: $alt
        ) {
            ftm_belongs_to_ft_id
            ftm_alt
            ftm_num
            ftm_prev_alt
            ftm_role
            ftm_content
            ftm_tool_calls
            ftm_call_id
            ftm_usage
            ftm_created_ts
        }
    }
    "#;

    let variables = json!({
        "thread_id": thread_id,
        "alt": alt
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

        // Check for GraphQL errors
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }

        // Extract the messages
        if let Some(data) = response_json.get("data") {
            if let Some(messages) = data.get("thread_messages_by_alt") {
                if let Some(messages_array) = messages.as_array() {
                    let messages: Vec<ThreadMessage> = serde_json::from_value(messages.clone())
                        .map_err(|e| format!("Failed to parse thread messages: {}", e))?;
                    
                    info!("Successfully retrieved {} messages for thread {} with alt={}", 
                          messages.len(), thread_id, alt);
                    return Ok(messages);
                }
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
            "Failed to get thread messages: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}


