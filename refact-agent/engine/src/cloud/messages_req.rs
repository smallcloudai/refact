use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall, ChatUsage};
use crate::global_context::GlobalContext;
use crate::scratchpads::passthrough_convert_messages;
use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadMessage {
    pub ftm_belongs_to_ft_id: String,
    pub ftm_alt: i32,
    pub ftm_num: i32,
    pub ftm_prev_alt: i32,
    pub ftm_role: String,
    pub ftm_content: Option<Value>,
    pub ftm_tool_calls: Option<Value>,
    pub ftm_call_id: String,
    pub ftm_usage: Option<Value>,
    pub ftm_created_ts: f64,
    pub ftm_provenance: Value,
}

pub async fn get_thread_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    alt: i64,
) -> Result<Vec<ThreadMessage>, String> {
    let client = Client::new();
    let api_key = crate::cloud::constants::API_KEY;
    let query = r#"
    query GetThreadMessagesByAlt($thread_id: String!, $alt: Int!) {
        thread_messages_list(
            ft_id: $thread_id,
            ftm_alt: $alt
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
            ftm_provenance
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
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(format!("GraphQL error: {}", error_msg));
        }
        if let Some(data) = response_json.get("data") {
            if let Some(messages) = data.get("thread_messages_list") {
                let messages: Vec<ThreadMessage> = serde_json::from_value(messages.clone())
                    .map_err(|e| format!("Failed to parse thread messages: {}", e))?;
                return Ok(messages);
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

pub async fn create_thread_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_id: &str,
    messages: Vec<ThreadMessage>,
) -> Result<(), String> {
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
            "ftm_content": serde_json::to_string(&message.ftm_content).unwrap(),
            "ftm_tool_calls": tool_calls_str,
            "ftm_call_id": message.ftm_call_id,
            "ftm_usage": usage_str,
            "ftm_provenance": serde_json::to_string(&message.ftm_provenance).unwrap()
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
        if let Some(_) = response_json.get("data") {
            return Ok(())
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

pub fn convert_thread_messages_to_messages(
    thread_messages: &Vec<ThreadMessage>,
) -> Vec<ChatMessage> {
    thread_messages
        .iter()
        .map(|msg| {
            let content: ChatContent = if let Some(content) = &msg.ftm_content {
                serde_json::from_value(content.clone()).unwrap_or_default()
            } else {
                ChatContent::default()
            };
            tracing::warn!("{:?}", msg.ftm_tool_calls);
            let tool_calls = msg.ftm_tool_calls.clone().map(|tc| {
                match serde_json::from_value::<Vec<ChatToolCall>>(tc) {
                    Ok(calls) => calls,
                    Err(_) => vec![],
                }
            });

            ChatMessage {
                role: msg.ftm_role.clone(),
                content,
                tool_calls,
                tool_call_id: msg.ftm_call_id.clone(),
                tool_failed: None,
                usage: msg.ftm_usage.clone().map(|u| {
                    match serde_json::from_value::<ChatUsage>(u) {
                        Ok(usage) => usage,
                        Err(_) => ChatUsage::default(),
                    }
                }),
                checkpoints: vec![],
                thinking_blocks: None,
                finish_reason: None,
            }
        })
        .collect()
}

pub fn convert_messages_to_thread_messages(
    messages: Vec<ChatMessage>,
    alt: i32,
    prev_alt: i32,
    start_num: i32,
    thread_id: &str,
) -> Result<Vec<ThreadMessage>, String> {
    let openai_messages = passthrough_convert_messages::convert_messages_to_openai_format(
        messages.clone(),
        &None,
        "",
    );
    let mut output_messages = vec![];
    for (i, msg) in messages.into_iter().enumerate() {
        let num = start_num + i as i32;
        let openai_msg = &openai_messages[i];
        let tool_calls = if let Some(tc) = &msg.tool_calls {
            Some(serde_json::to_value(tc).unwrap_or(Value::Null))
        } else {
            None
        };
        let usage = msg
            .usage
            .map(|u| serde_json::to_value(u).unwrap_or(Value::Null));
        let role = openai_msg
            .get("role")
            .map(|x| x.as_str().unwrap().to_string())
            .ok_or("cannot find role in the message".to_string())?;
        let content = openai_msg
            .get("content")
            .cloned()
            .ok_or("cannot find role in the message".to_string())?;
        output_messages.push(ThreadMessage {
            ftm_belongs_to_ft_id: thread_id.to_string(),
            ftm_alt: alt,
            ftm_num: num,
            ftm_prev_alt: prev_alt,
            ftm_role: role,
            ftm_content: Some(content),
            ftm_tool_calls: tool_calls,
            ftm_call_id: msg.tool_call_id,
            ftm_usage: usage,
            ftm_created_ts: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            ftm_provenance: json!({"important": "information"}),
        })
    }
    Ok(output_messages)
}

pub async fn get_tool_names_from_openai_format(
    toolset_json: &Vec<Value>,
) -> Result<Vec<String>, String> {
    let mut tool_names = Vec::new();
    for tool in toolset_json {
        if let Some(function) = tool.get("function") {
            if let Some(name) = function.get("name") {
                if let Some(name_str) = name.as_str() {
                    tool_names.push(name_str.to_string());
                }
            }
        }
    }
    Ok(tool_names)
}
