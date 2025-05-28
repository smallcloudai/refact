use crate::at_commands::at_commands::AtCommandsContext;
use crate::caps::resolve_chat_model;
use crate::cloud::threads_req::{Thread, ThreadMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::scratchpads::chat_utils_prompts::system_prompt_add_extra_instructions;
use crate::scratchpads::scratchpad_utils::HasRagResults;
use crate::tokens;
use crate::tools::tools_description::{tool_description_list_from_yaml, tools_merged_and_filtered, Tool};
use crate::tools::tools_execute::run_tools_locally;
use crate::call_validation::{ChatMessage, ChatContent, ChatToolCall, ChatUsage};
use crate::custom_error::MapErrToString;
use futures::{SinkExt, StreamExt};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message
};
use tracing::{error, info};
use tracing_subscriber::fmt::format::json;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub owner_fuser_id: String,
    pub owner_shared: bool,
    pub ft_id: String,
    pub ft_title: String,
    pub ft_error: String,
    pub ft_updated_ts: f64,
    pub ft_locked_by: String,
    pub ft_need_assistant: i64,
    pub ft_need_tool_calls: i64,
    pub ft_anything_new: bool,
    pub ft_archived_ts: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NewsAction {
    Insert,
    Update,
    Delete,
    #[serde(rename = "INITIAL_UPDATES_OVER")]
    InitialUpdatesOver,
    #[serde(other)]
    Unknown,
}

impl NewsAction {
    pub fn to_lowercase(&self) -> String {
        match self {
            NewsAction::Insert => "insert".to_string(),
            NewsAction::Update => "update".to_string(),
            NewsAction::Delete => "delete".to_string(),
            NewsAction::InitialUpdatesOver => "initial_updates_over".to_string(),
            NewsAction::Unknown => "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadEvent {
    #[serde(with = "news_action_string")]
    pub news_action: NewsAction,
    pub news_payload_id: String,
    pub news_payload: Option<ThreadPayload>,
}

// Custom serialization/deserialization for NewsAction
mod news_action_string {
    use super::NewsAction;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(action: &NewsAction, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match action {
            NewsAction::Insert => "INSERT",
            NewsAction::Update => "UPDATE",
            NewsAction::Delete => "DELETE",
            NewsAction::InitialUpdatesOver => "INITIAL_UPDATES_OVER",
            NewsAction::Unknown => "UNKNOWN",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NewsAction, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "INSERT" => Ok(NewsAction::Insert),
            "UPDATE" => Ok(NewsAction::Update),
            "DELETE" => Ok(NewsAction::Delete),
            "INITIAL_UPDATES_OVER" => Ok(NewsAction::InitialUpdatesOver),
            _ => Ok(NewsAction::Unknown),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadsInGroup {
    pub threads_in_group: ThreadEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLData {
    pub data: ThreadsInGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Data,
    Error,
    Complete,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    #[serde(rename = "type")]
    #[serde(with = "response_type_string")]
    pub response_type: ResponseType,
    pub payload: Option<GraphQLData>,
    pub id: Option<String>,
}

// Custom serialization/deserialization for ResponseType to maintain compatibility
// with the existing GraphQL protocol that uses strings
mod response_type_string {
    use super::ResponseType;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(response_type: &ResponseType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match response_type {
            ResponseType::Data => "data",
            ResponseType::Error => "error",
            ResponseType::Complete => "complete",
            ResponseType::Unknown => "unknown",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ResponseType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "data" => Ok(ResponseType::Data),
            "error" => Ok(ResponseType::Error),
            "complete" => Ok(ResponseType::Complete),
            _ => Ok(ResponseType::Unknown),
        }
    }
}

const THREADS_SUBSCRIPTION_QUERY : &str = r#"
    subscription ThreadsPageSubs($located_fgroup_id: String!, $limit: Int!) {
      threads_in_group(located_fgroup_id: $located_fgroup_id, limit: $limit) {
        news_action
        news_payload_id
        news_payload {
          owner_fuser_id
          owner_shared
          ft_id
          ft_title
          ft_error
          ft_updated_ts
          ft_locked_by
          ft_need_assistant
          ft_need_tool_calls
          ft_anything_new
          ft_archived_ts
        }
      }
    }
"#;


pub async fn watch_threads_subscription(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let located_fgroup_id = crate::cloud::constants::DEFAULT_FGROUP_ID;
    let thread_limit = crate::cloud::constants::DEFAULT_LIMIT;
    
    info!("Starting GraphQL subscription for threads_in_group with fgroup_id=\"{}\" and limit={}", 
          located_fgroup_id, thread_limit);
    
    loop {
        let mut connection = match initialize_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to initialize connection: {}", err);
                return;
            }
        };
        match events_loop(gcx.clone(), &mut connection).await {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to process events: {}", err);
                return;
            }
        }
    }
}


async fn initialize_connection() -> Result<futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>, String> {
    let url = Url::parse(crate::cloud::constants::GRAPHQL_WS_URL)
        .map_err(|e| format!("Failed to parse WebSocket URL: {}", e))?;
    let mut request = url.into_client_request()
        .map_err(|e| format!("Failed to create request: {}", e))?;
    request.headers_mut().insert("Sec-WebSocket-Protocol", "graphql-ws".parse().unwrap());
    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| format!("Failed to connect to WebSocket server: {}", e))?;
    let (mut write, mut read) = ws_stream.split();
    let init_message = json!({
        "type": "connection_init",
        "payload": {
            "apikey": crate::cloud::constants::API_KEY
        }
    });
    
    info!("Sending connection initialization message");
    write.send(Message::Text(init_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send connection init message: {}", e))?;
    
    let timeout = tokio::time::timeout(Duration::from_secs(5), read.next()).await
        .map_err(|_| "Timeout waiting for connection acknowledgment".to_string())?;
    
    if let Some(msg) = timeout {
        let msg = msg.map_err(|e| format!("WebSocket error: {}", e))?;
        match msg {
            Message::Text(text) => {
                info!("Received response: {}", text);
                let response: Value = serde_json::from_str(&text)
                    .map_err(|e| format!("Failed to parse connection response: {}", e))?;
                if let Some(msg_type) = response["type"].as_str() {
                    if msg_type == "connection_ack" {
                    } else if msg_type == "connection_error" {
                        return Err(format!("Connection error: {}", response));
                    } else {
                        return Err(format!("Expected connection_ack, got: {}", response));
                    }
                } else {
                    return Err(format!("Invalid response format, missing 'type': {}", response));
                }
            },
            Message::Close(frame) => {
                return if let Some(frame) = frame {
                    Err(format!("WebSocket closed during initialization: code={}, reason={}",
                                frame.code, frame.reason))
                } else {
                    Err("WebSocket connection closed during initialization without details".to_string())
                }
            },
            _ => {
                return Err(format!("Unexpected message type received: {:?}", msg));
            }
        }
    } else {
        return Err("No response received for connection initialization".to_string());
    }
    
    let subscription_message = json!({
        "id": "42",
        "type": "start",
        "payload": {
            "query": THREADS_SUBSCRIPTION_QUERY,
            "variables": {
                "located_fgroup_id": crate::cloud::constants::DEFAULT_FGROUP_ID,
                "limit": crate::cloud::constants::DEFAULT_LIMIT
            }
        }
    });

    write.send(Message::Text(subscription_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription message: {}", e))?;
    
    Ok(read)
}


async fn process_thread_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload)-> Result<(), String> {
    let messages = crate::cloud::threads_req::get_thread_messages(
        gcx.clone(), &thread_payload.ft_id, thread_payload.ft_need_tool_calls
    ).await?;
    let thread = crate::cloud::threads_req::get_thread(gcx.clone(), &thread_payload.ft_id).await?;
    if messages.iter().any(|x| x.ftm_role != "system") {
        initialize_thread(gcx.clone(), &thread.ft_fexp_name, &thread, &messages).await?;
    } else {
        call_tools(gcx.clone(), &thread, &messages).await?;
    }
    Ok(())
}

async fn initialize_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    expert_name: &str,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>
) -> Result<(), String> {
    let expert = crate::cloud::experts_req::get_expert(gcx.clone(), expert_name).await?;
    let tools: IndexMap<String, Box<dyn Tool + Send>> = tools_merged_and_filtered(gcx.clone(), true).await?
        .into_iter()
        .filter(|(name, _tool)| expert.is_tool_allowed(name))
        .collect();
    let tool_descriptions = tool_description_list_from_yaml(tools, None, true).await?
        .into_iter()
        .map(|x| x.into_openai_style())
        .collect::<Vec<_>>();
    let updated_system_prompt = system_prompt_add_extra_instructions(
        gcx.clone(),
        &expert.fexp_system_prompt
    ).await;

    let last_message = thread_messages.last().unwrap();
    crate::cloud::threads_req::update_thread(gcx.clone(), &thread.ft_id, tool_descriptions).await?;
    
    let output_thread_messages = vec![ThreadMessage {
        ftm_belongs_to_ft_id: last_message.ftm_belongs_to_ft_id.clone(),
        ftm_alt: last_message.ftm_alt.clone(),
        ftm_num: 0,
        ftm_prev_alt: 100,
        ftm_role: "system".to_string(),
        ftm_content: serde_json::to_value(ChatContent::SimpleText(updated_system_prompt)).unwrap(),
        ftm_tool_calls: None,
        ftm_call_id: "".to_string(),
        ftm_usage: None,
        ftm_created_ts: std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs_f64(),
        ftm_provenance: json!({}),
    }];
    crate::cloud::messages_req::create_thread_messages(gcx.clone(), &thread.ft_id, output_thread_messages).await?;

    Ok(())
}

fn convert_thread_messages_to_messages(thread_messages: &Vec<ThreadMessage>) -> Vec<ChatMessage> {
    thread_messages.iter().map(|msg| {
        let content: ChatContent = serde_json::from_value(msg.ftm_content.clone()).unwrap();
        let tool_calls = msg.ftm_tool_calls.clone().map(|tc| {
            match serde_json::from_value::<Vec<ChatToolCall>>(tc) {
                Ok(calls) => calls,
                Err(_) => vec![]
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
                    Err(_) => ChatUsage::default()
                }
            }),
            checkpoints: vec![],
            thinking_blocks: None,
            finish_reason: None,
        }
    }).collect()
}

fn convert_messages_to_thread_messages(messages: Vec<ChatMessage>, alt: i32, prev_alt: i32, start_num: i32, thread_id: &str) -> Vec<ThreadMessage> {
    messages.into_iter().enumerate().map(|(i, msg)| {
        let num = start_num + i as i32;
        let tool_calls = if let Some(tc) = &msg.tool_calls {
            Some(serde_json::to_value(tc).unwrap_or(Value::Null))
        } else {
            None
        };
        
        let usage = msg.usage.map(|u| serde_json::to_value(u).unwrap_or(Value::Null));
        let content = serde_json::to_value(msg.content).unwrap();
        
        ThreadMessage {
            ftm_belongs_to_ft_id: thread_id.to_string(),
            ftm_alt: alt,
            ftm_num: num,
            ftm_prev_alt: prev_alt,
            ftm_role: msg.role,
            ftm_content: content,
            ftm_tool_calls: tool_calls,
            ftm_call_id: msg.tool_call_id,
            ftm_usage: usage,
            ftm_created_ts: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            ftm_provenance: json!({}),
        }
    }).collect()
}

async fn get_tool_names_from_openai_format(toolset_json: &Vec<Value>) -> Result<Vec<String>, String> {
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

async fn call_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>
) -> Result<(), String> {
    let max_new_tokens = 8192;
    let last_message_num = thread_messages.iter().map(|x| x.ftm_num).max().unwrap_or(0);
    let (alt, prev_alt) = thread_messages.last()
        .map(|msg| (msg.ftm_alt, msg.ftm_prev_alt))
        .unwrap_or((0, 0));
    let messages = convert_thread_messages_to_messages(thread_messages);
    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(), 
        32000,
        12,
        false,
        messages.clone(),
        thread.ft_id.to_string(),
        false,
        thread.ft_model.to_string(),
    ).await));
    let allowed_tools = get_tool_names_from_openai_format(&thread.ft_toolset).await?;
    let mut all_tools: IndexMap<String, Box<dyn Tool + Send>> = tools_merged_and_filtered(gcx.clone(), true).await?
        .into_iter()
        .filter(|(name, _)| allowed_tools.contains(name))
        .collect();
    let mut has_rag_results = HasRagResults::new();
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err_to_string()?;
    let model_rec = resolve_chat_model(caps, &thread.ft_model)
        .map_err(|e| format!("Failed to resolve chat model: {}", e))?;
    let tokenizer_arc = tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await?;
    let (messages, _) = run_tools_locally(
        ccx.clone(),
        &mut all_tools,
        tokenizer_arc,
        max_new_tokens,
        &messages,
        &mut has_rag_results, 
        &None
    ).await?;
    let output_thread_messages = convert_messages_to_thread_messages(messages, alt, prev_alt, last_message_num + 1, &thread.ft_id);
    crate::cloud::messages_req::create_thread_messages(gcx.clone(), &thread.ft_id, output_thread_messages).await?;
    Ok(())
}


async fn events_loop(
    gcx: Arc<ARwLock<GlobalContext>>,
    connection: &mut futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>
) -> Result<(), String> {
    info!("Cloud threads subscription started, waiting for events...");
    
    while let Some(msg) = connection.next().await {
        if gcx.read().await.shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Shutting down GraphQL subscription thread");
            break;
        }
        
        match msg {
            Ok(Message::Text(text)) => {
                let response = match serde_json::from_str::<GraphQLResponse>(&text) {
                    Ok(res) => res,
                    Err(err) => {
                        error!("Failed to parse message: {}, error: {}", text, err);
                        continue;
                    }
                };
                match response.response_type {
                    ResponseType::Data => {
                        if let Some(input) = response.payload {
                            if input.data.threads_in_group.news_action != NewsAction::Insert &&
                                input.data.threads_in_group.news_action != NewsAction::Update {
                                continue;
                            }
                            if let Some(payload) = input.data.threads_in_group.news_payload {
                                match process_thread_event(gcx.clone(), &payload).await {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!("Failed to process thread event: {}", err);
                                    }
                                }
                            }
                        } else {
                            info!("Received data message but couldn't find payload");
                        }
                    },
                    ResponseType::Error => {
                        error!("Subscription error: {}", text);
                    },
                    ResponseType::Complete => {
                        info!("Subscription completed");
                        break;
                    }
                    ResponseType::Unknown => {
                        info!("Received message with unknown type: {}", text);
                    }
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                return Err(format!("WebSocket error: {}", e));
            }
        }
    }
    
    Ok(())
}
