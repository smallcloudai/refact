use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use crate::cloud::threads_req::ThreadMessage;
use crate::global_context::GlobalContext;

#[derive(Debug)]
pub struct ThreadMessagesCreateResult {
    pub count: usize,
    pub message_ids: Vec<String>,
}

pub async fn create_thread_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    messages: Vec<ThreadMessage>,
) -> Result<ThreadMessagesCreateResult, String> {
    if messages.is_empty() {
        return Err("No messages provided".to_string());
    }

    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;
    let mut input_messages = Vec::with_capacity(messages.len());

    for message in messages {
        if message.ftm_belongs_to_ft_id != thread_id {
            return Err(format!(
                "Message thread ID {} doesn't match the provided thread ID {}",
                message.ftm_belongs_to_ft_id, thread_id
            ));
        }
        if message.ftm_role.is_empty() {
            return Err("Message role is required".to_string());
        }

        let content_str = serde_json::to_string(&message.ftm_content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;

        let tool_calls_str = match &message.ftm_tool_calls {
            Some(tc) => serde_json::to_string(tc)
                .map_err(|e| format!("Failed to serialize tool_calls: {}", e))?,
            None => "{}".to_string(),
        };
        let usage_str = match &message.ftm_usage {
            Some(u) => {
                serde_json::to_string(u).map_err(|e| format!("Failed to serialize usage: {}", e))?
            }
            None => "{}".to_string(),
        };

        input_messages.push(json!({
            "ftm_belongs_to_ft_id": message.ftm_belongs_to_ft_id,
            "ftm_alt": message.ftm_alt,
            "ftm_num": message.ftm_num,
            "ftm_prev_alt": message.ftm_prev_alt,
            "ftm_role": message.ftm_role,
            "ftm_content": content_str,
            "ftm_tool_calls": tool_calls_str,
            "ftm_call_id": message.ftm_call_id,
            "ftm_usage": usage_str,
            "ftm_provenance": message.ftm_provenance
        }));
    }

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
                ftm_provenance
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

        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }

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
