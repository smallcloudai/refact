use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

#[derive(Debug)]
pub struct ThreadMessagesCreateResult {
    pub count: usize,
    pub message_ids: Vec<String>,
}

pub struct ThreadMessageRequest {
    pub thread_id: String,
    pub role: String,
    pub content: Value,
    pub tool_calls: Option<Value>,
    pub num: i32,
    pub alt: i32,
    pub prev_alt: i32,
    pub call_id: Option<String>,
    pub usage: Option<Value>,
}

impl Default for ThreadMessageRequest {
    fn default() -> Self {
        Self {
            thread_id: String::new(),
            role: String::new(),
            content: json!({}),
            tool_calls: None,
            num: 0,
            alt: 0,
            prev_alt: 0,
            call_id: None,
            usage: None,
        }
    }
}

pub async fn create_thread_messages_multiple_req(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    messages: Vec<ThreadMessageRequest>,
) -> Result<ThreadMessagesCreateResult, String> {
    if messages.is_empty() {
        return Err("No messages provided".to_string());
    }

    let client = Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();

    // Prepare the input messages
    let mut input_messages = Vec::with_capacity(messages.len());

    for message in messages {
        // Validate each message
        if message.thread_id != thread_id {
            return Err(format!(
                "Message thread ID {} doesn't match the provided thread ID {}",
                message.thread_id, thread_id
            ));
        }
        if message.role.is_empty() {
            return Err("Message role is required".to_string());
        }

        let call_id = message
            .call_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Convert content to string
        let content_str = serde_json::to_string(&message.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;

        // Convert tool_calls to string if present
        let tool_calls_str = match &message.tool_calls {
            Some(tc) => serde_json::to_string(tc)
                .map_err(|e| format!("Failed to serialize tool_calls: {}", e))?,
            None => "{}".to_string(),
        };

        // Convert usage to string if present
        let usage_str = match &message.usage {
            Some(u) => {
                serde_json::to_string(u).map_err(|e| format!("Failed to serialize usage: {}", e))?
            }
            None => "{}".to_string(),
        };

        // Add the message to the input list
        input_messages.push(json!({
            "ftm_belongs_to_ft_id": thread_id,
            "ftm_alt": message.alt,
            "ftm_num": message.num,
            "ftm_prev_alt": message.prev_alt,
            "ftm_role": message.role,
            "ftm_content": content_str,
            "ftm_tool_calls": tool_calls_str,
            "ftm_call_id": call_id,
            "ftm_usage": usage_str
        }));
    }

    // Construct the GraphQL mutation
    let variables = json!({
        "input": {
            "ftm_belongs_to_ft_id": thread_id,
            "messages": input_messages
        }
    });

    let mutation = r#"
    mutation ThreadMessagesCreateMultiple($input: FThreadMultipleMessagesInput!) {
        thread_messages_create_multiple(input: $input) {
            count
            messages {
                ftm_belongs_to_ft_id
                ftm_alt
                ftm_num
                ftm_prev_alt
                ftm_role
                ftm_created_ts
                ftm_call_id
            }
        }
    }
    "#;

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

        // Extract the created messages
        if let Some(data) = response_json.get("data") {
            if let Some(result) = data.get("thread_messages_create_multiple") {
                if let Some(count) = result.get("count").and_then(|c| c.as_u64()) {
                    let mut message_ids = Vec::new();

                    if let Some(messages) = result.get("messages").and_then(|m| m.as_array()) {
                        for msg in messages {
                            if let Some(id) =
                                msg.get("ftm_belongs_to_ft_id").and_then(|id| id.as_str())
                            {
                                message_ids.push(id.to_string());
                            }
                        }
                    }

                    info!(
                        "Successfully created {} messages in thread {}",
                        count, thread_id
                    );
                    return Ok(ThreadMessagesCreateResult {
                        count: count as usize,
                        message_ids,
                    });
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
            "Failed to create thread messages: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}

pub async fn create_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    conversation: Vec<(&str, &str)>,
) -> Result<ThreadMessagesCreateResult, String> {
    if conversation.is_empty() {
        return Err("No conversation messages provided".to_string());
    }

    let mut messages = Vec::with_capacity(conversation.len());

    for (i, (role, content)) in conversation.iter().enumerate() {
        messages.push(ThreadMessageRequest {
            thread_id: thread_id.to_string(),
            role: role.to_string(),
            content: json!({ "text": content }),
            num: i as i32,
            alt: 0,
            prev_alt: 0,
            ..Default::default()
        });
    }

    create_thread_messages_multiple_req(gcx, thread_id, messages).await
}
