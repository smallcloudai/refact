use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall, DiffChunk};
use crate::cloud::graphql_client::{execute_graphql, execute_graphql_no_result, GraphQLRequestConfig, graphql_error_to_string};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use itertools::Itertools;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub ftm_user_preferences: Option<Value>
}

pub async fn get_thread_messages(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    alt: i64,
) -> Result<Vec<ThreadMessage>, String> {
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
            ftm_user_preferences
        }
    }
    "#;
    let variables = json!({
        "thread_id": thread_id,
        "alt": alt
    });
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        user_agent: Some("refact-lsp".to_string()),
        additional_headers: None,
    };

    execute_graphql::<Vec<ThreadMessage>, _>(
        config, 
        query, 
        variables, 
        "thread_messages_list"
    )
    .await
    .map_err(graphql_error_to_string)
}

pub async fn create_thread_messages(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    messages: Vec<ThreadMessage>,
) -> Result<(), String> {
    if messages.is_empty() {
        return Err("No messages provided".to_string());
    }
    
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
            "ftm_provenance": serde_json::to_string(&message.ftm_provenance).unwrap(),
            "ftm_user_preferences": serde_json::to_string(&message.ftm_user_preferences).unwrap()
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
                ftm_user_preferences
            }
        }
    }
    "#;
    
    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        user_agent: Some("refact-lsp".to_string()),
        additional_headers: None,
    };

    execute_graphql_no_result(
        config, 
        mutation, 
        variables, 
        "thread_messages_create_multiple"
    )
    .await
    .map_err(graphql_error_to_string)
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
            let tool_calls = msg.ftm_tool_calls.clone().map(|tc| {
                serde_json::from_value::<Vec<ChatToolCall>>(tc).unwrap_or_else(|_| vec![])
            });
            ChatMessage {
                role: msg.ftm_role.clone(),
                content,
                tool_calls,
                tool_call_id: msg.ftm_call_id.clone(),
                tool_failed: None,
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
    user_preferences: Option<Value>,
) -> Result<Vec<ThreadMessage>, String> {
    let mut output_messages = Vec::new();
    let flush_delayed_images = |results: &mut Vec<Value>, delay_images: &mut Vec<Value>| {
        results.extend(delay_images.clone());
        delay_images.clear();
    };
    for (i, msg) in messages.into_iter().enumerate() {
        let num = start_num + i as i32;
        let mut delay_images = vec![];
        let mut messages = if msg.role == "tool" {
            let mut results = vec![];
            match &msg.content {
                ChatContent::Multimodal(multimodal_content) => {
                    let texts = multimodal_content.iter().filter(|x|x.is_text()).collect::<Vec<_>>();
                    let images = multimodal_content.iter().filter(|x|x.is_image()).collect::<Vec<_>>();
                    let text = if texts.is_empty() {
                        "attached images below".to_string()
                    } else {
                        texts.iter().map(|x|x.m_content.clone()).collect::<Vec<_>>().join("\n")
                    };
                    let mut msg_cloned = msg.clone();
                    msg_cloned.content = ChatContent::SimpleText(text);
                    results.push(msg_cloned.into_value(&None, ""));
                    if !images.is_empty() {
                        let msg_img = ChatMessage {
                            role: "user".to_string(),
                            content: ChatContent::Multimodal(images.into_iter().cloned().collect()),
                            ..Default::default()
                        };
                        delay_images.push(msg_img.into_value(&None, ""));
                    }
                },
                ChatContent::SimpleText(_) => {
                    results.push(msg.into_value(&None, ""));
                }
            }
            results
        } else if msg.role == "assistant" || msg.role == "system" {
            vec![msg.into_value(&None, "")]
        } else if msg.role == "user" {
            vec![msg.into_value(&None, "")]
        } else if msg.role == "diff" {
            let extra_message = match serde_json::from_str::<Vec<DiffChunk>>(&msg.content.content_text_only()) {
                Ok(chunks) => {
                    if chunks.is_empty() {
                        "Nothing has changed.".to_string()
                    } else {
                        chunks.iter()
                            .filter(|x| !x.application_details.is_empty())
                            .map(|x| x.application_details.clone())
                            .join("\n")
                    }
                },
                Err(_) => "".to_string()
            };
            vec![ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText(format!("The operation has succeeded.\n{extra_message}")),
                tool_calls: None,
                tool_call_id: msg.tool_call_id.clone(),
                ..Default::default()
            }.into_value(&None, "")]
        } else if msg.role == "plain_text" || msg.role == "cd_instruction" || msg.role == "context_file" {
            vec![ChatMessage::new(
                msg.role.to_string(),
                msg.content.content_text_only(),
            ).into_value(&None, "")]
        } else {
            warn!("unknown role: {}", msg.role);
            vec![]
        };
        flush_delayed_images(&mut messages, &mut delay_images);
        for pp_msg in messages {
            let tool_calls = pp_msg.get("tool_calls")
                .map(|x| Some(x.clone())).unwrap_or(None);
            let usage = pp_msg.get("usage")
                .map(|x| Some(x.clone())).unwrap_or(None);
            let content = pp_msg
                .get("content")
                .cloned()
                .ok_or("cannot find role in the message".to_string())?;
            output_messages.push(ThreadMessage {
                ftm_belongs_to_ft_id: thread_id.to_string(),
                ftm_alt: alt,
                ftm_num: num,
                ftm_prev_alt: prev_alt,
                ftm_role: msg.role.clone(),
                ftm_content: Some(content),
                ftm_tool_calls: tool_calls,
                ftm_call_id: msg.tool_call_id.clone(),
                ftm_usage: usage,
                ftm_created_ts: std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                ftm_provenance: json!({"system_type": "refact_lsp", "version": env!("CARGO_PKG_VERSION") }),
                ftm_user_preferences: user_preferences.clone(),
            })
        }
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
