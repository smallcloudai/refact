use std::collections::HashSet;
use crate::global_context::GlobalContext;
use futures::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatContent;
use crate::cloud::messages_req::ThreadMessage;
use crate::cloud::threads_req::{lock_thread, Thread};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use crate::custom_error::MapErrToString;


const RECONNECT_DELAY_SECONDS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub owner_fuser_id: String,
    pub ft_id: String,
    pub ft_error: Option<String>,
    pub ft_locked_by: String,
    pub ft_need_tool_calls: i64,
    pub ft_app_searchable: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicStuff {
    pub fuser_id: String,
    pub workspaces: Vec<Value>,
}

const THREADS_SUBSCRIPTION_QUERY: &str = r#"
    subscription ThreadsPageSubs($located_fgroup_id: String!) {
      threads_in_group(located_fgroup_id: $located_fgroup_id) {
        news_action
        news_payload_id
        news_payload {
          owner_fuser_id
          ft_id
          ft_error
          ft_locked_by
          ft_need_tool_calls
          ft_app_searchable
        }
      }
    }
"#;

pub async fn trigger_threads_subscription_restart(gcx: Arc<ARwLock<GlobalContext>>) {
    let restart_flag = gcx.read().await.threads_subscription_restart_flag.clone();
    restart_flag.store(true, Ordering::SeqCst);
    info!("threads subscription restart triggered");
}

pub async fn watch_threads_subscription(gcx: Arc<ARwLock<GlobalContext>>) {
    if !gcx.read().await.cmdline.cloud_threads {
        return;
    }
    
    loop {
        {
            let restart_flag = gcx.read().await.threads_subscription_restart_flag.clone();
            restart_flag.store(false, Ordering::SeqCst);
        }
        let located_fgroup_id = if let Some(located_fgroup_id) = gcx.read().await.active_group_id.clone() {
            located_fgroup_id
        } else {
            warn!("no active group is set, skipping threads subscription");
            tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
            continue;
        };
        
        info!(
            "starting subscription for threads_in_group with fgroup_id=\"{}\"",
            located_fgroup_id
        );
        let connection_result = initialize_connection(gcx.clone()).await;
        let mut connection = match connection_result {
            Ok(conn) => conn,
            Err(err) => {
                error!("failed to initialize connection: {}", err);
                info!("will attempt to reconnect in {} seconds", RECONNECT_DELAY_SECONDS);
                tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
                continue;
            }
        };
        
        let events_result = events_loop(gcx.clone(), &mut connection).await;
        if let Err(err) = events_result {
            error!("failed to process events: {}", err);
            info!("will attempt to reconnect in {} seconds", RECONNECT_DELAY_SECONDS);
        }
        
        if gcx.read().await.shutdown_flag.load(Ordering::SeqCst) {
            info!("shutting down threads subscription");
            break;
        }
        
        let restart_flag = gcx.read().await.threads_subscription_restart_flag.clone();
        if !restart_flag.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
        }
    }
}

async fn initialize_connection(gcx: Arc<ARwLock<GlobalContext>>) -> Result<
    futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
    >,
    String,
> {
    let (api_key, located_fgroup_id) = {
        let gcx_read = gcx.read().await;
        (gcx_read.cmdline.api_key.clone(), 
         gcx_read.active_group_id.clone().unwrap_or_default())
    };
    let url = Url::parse(crate::cloud::constants::GRAPHQL_WS_URL)
        .map_err(|e| format!("Failed to parse WebSocket URL: {}", e))?;
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("Failed to create request: {}", e))?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "graphql-ws".parse().unwrap());
    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| format!("Failed to connect to WebSocket server: {}", e))?;
    let (mut write, mut read) = ws_stream.split();
    let init_message = json!({
        "type": "connection_init",
        "payload": {
            "apikey": api_key
        }
    });
    write.send(Message::Text(init_message.to_string())).await
        .map_err(|e| format!("Failed to send connection init message: {}", e))?;

    let timeout = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
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
                    return Err(format!(
                        "Invalid response format, missing 'type': {}",
                        response
                    ));
                }
            }
            Message::Close(frame) => {
                return if let Some(frame) = frame {
                    Err(format!(
                        "WebSocket closed during initialization: code={}, reason={}",
                        frame.code, frame.reason
                    ))
                } else {
                    Err(
                        "WebSocket connection closed during initialization without details"
                            .to_string(),
                    )
                }
            }
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
                "located_fgroup_id": located_fgroup_id
            }
        }
    });
    write
        .send(Message::Text(subscription_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription message: {}", e))?;
    Ok(read)
}

async fn events_loop(
    gcx: Arc<ARwLock<GlobalContext>>,
    connection: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<(), String> {
    info!("cloud threads subscription started, waiting for events...");
    let basic_info = get_basic_info(gcx.clone()).await?;
    while let Some(msg) = connection.next().await {
        if gcx.read().await.shutdown_flag.load(Ordering::SeqCst) {
            info!("shutting down threads subscription");
            break;
        }
        if gcx.read().await.threads_subscription_restart_flag.load(Ordering::SeqCst) {
            info!("restart flag detected, restarting threads subscription");
            return Ok(());
        }
        match msg {
            Ok(Message::Text(text)) => {
                let response: Value = match serde_json::from_str(&text) {
                    Ok(res) => res,
                    Err(err) => {
                        error!("failed to parse message: {}, error: {}", text, err);
                        continue;
                    }
                };
                let response_type = response["type"].as_str().unwrap_or("unknown");
                match response_type {
                    "data" => {
                        if let Some(payload) = response["payload"].as_object() {
                            let data = &payload["data"];
                            let threads_in_group = &data["threads_in_group"];
                            let news_action = threads_in_group["news_action"].as_str().unwrap_or("");
                            if news_action != "INSERT" && news_action != "UPDATE" {
                                continue;
                            }
                            if let Ok(payload) = serde_json::from_value::<ThreadPayload>(threads_in_group["news_payload"].clone()) {
                                match process_thread_event(gcx.clone(), &payload, &basic_info).await {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!("failed to process thread event: {}", err);
                                    }
                                }
                            } else {
                                info!("failed to parse thread payload: {}", text);
                            }
                        } else {
                            info!("received data message but couldn't find payload");
                        }
                    }
                    "error" => {
                        error!("threads subscription error: {}", text);
                    }
                    _ => {
                        info!("received message with unknown type: {}", text);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("webSocket connection closed");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                return Err(format!("webSocket error: {}", e));
            }
        }
    }
    Ok(())
}
fn generate_random_hash(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

async fn get_basic_info(gcx: Arc<ARwLock<GlobalContext>>) -> Result<BasicStuff, String> {
    let client = Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let query = r#"
    query GetBasicInfo {
      query_basic_stuff {
        fuser_id
        workspaces {
          ws_id
          ws_owner_fuser_id
          ws_root_group_id
          root_group_name
        }
      }
    }
    "#;
    let response = client
        .post(&crate::cloud::constants::GRAPHQL_URL.to_string())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({"query": query}))
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
            return Err(format!("GraphQL request error: {}", error_msg));
        }
        if let Some(data) = response_json.get("data") {
            let basic_stuff_struct: BasicStuff = serde_json::from_value(data["query_basic_stuff"].clone())
                .map_err(|e| format!("Failed to parse updated thread: {}", e))?;
            return Ok(basic_stuff_struct);
        }
        Err(format!("Basic data not found or unexpected response format: {}", response_body))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to get basic data: HTTP status {}, error: {}",
            status, error_text
        ))
    }
}

async fn process_thread_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload,
    basic_info: &BasicStuff
) -> Result<(), String> {
    if thread_payload.ft_need_tool_calls == -1 || thread_payload.owner_fuser_id != basic_info.fuser_id {
        return Ok(());
    }
    let app_searchable_id = gcx.read().await.app_searchable_id.clone();
    if let Some(ft_app_searchable) = thread_payload.ft_app_searchable.clone() {
        if ft_app_searchable != app_searchable_id {
            info!("thread `{}` has different `app_searchable` id, skipping it", thread_payload.ft_id);
        }
    } else {
        info!("thread `{}` doesn't have the `app_searchable` id, skipping it", thread_payload.ft_id);
        return Ok(());
    }
    if let Some(error) = thread_payload.ft_error.as_ref() {
        info!("thread `{}` has the error: `{}`. Skipping it", thread_payload.ft_id, error);
        return Ok(());
    }
    let messages = crate::cloud::messages_req::get_thread_messages(
        gcx.clone(),
        &thread_payload.ft_id,
        thread_payload.ft_need_tool_calls,
    ).await?;
    let thread = crate::cloud::threads_req::get_thread(gcx.clone(), &thread_payload.ft_id).await?;
    let hash = generate_random_hash(16);
    match lock_thread(gcx.clone(), &thread.ft_id, &hash).await {
        Ok(_) => {}
        Err(err) => return Err(err)
    }
    let result = if messages.iter().all(|x| x.ftm_role != "system") {
        initialize_thread(gcx.clone(), &thread.ft_fexp_name, &thread, &messages).await
    } else {
        call_tools(gcx.clone(), &thread, &messages).await
    };
    match crate::cloud::threads_req::unlock_thread(gcx.clone(), thread.ft_id.clone(), hash).await {
        Ok(_) => info!("thread `{}` unlocked successfully", thread.ft_id),
        Err(err) => error!("failed to unlock thread `{}`: {}", thread.ft_id, err),
    }
    result
}

async fn initialize_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    expert_name: &str,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
) -> Result<(), String> {
    let expert = crate::cloud::experts_req::get_expert(gcx.clone(), expert_name).await?;
    let tools: Vec<Box<dyn crate::tools::tools_description::Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone())
            .await
            .into_iter()
            .filter(|tool| expert.is_tool_allowed(&tool.tool_description().name))
            .collect();
    let tool_descriptions = tools
        .iter()
        .map(|x| x.tool_description().into_openai_style())
        .collect::<Vec<_>>();
    crate::cloud::threads_req::set_thread_toolset(gcx.clone(), &thread.ft_id, tool_descriptions).await?;
    let updated_system_prompt = crate::scratchpads::chat_utils_prompts::system_prompt_add_extra_instructions(
        gcx.clone(), expert.fexp_system_prompt.clone(), HashSet::new()
    ).await;
    let last_message = thread_messages.last().unwrap();
    let output_thread_messages = vec![ThreadMessage {
        ftm_belongs_to_ft_id: last_message.ftm_belongs_to_ft_id.clone(),
        ftm_alt: last_message.ftm_alt.clone(),
        ftm_num: 0,
        ftm_prev_alt: 100,
        ftm_role: "system".to_string(),
        ftm_content: Some(
            serde_json::to_value(ChatContent::SimpleText(updated_system_prompt)).unwrap(),
        ),
        ftm_tool_calls: None,
        ftm_call_id: "".to_string(),
        ftm_usage: None,
        ftm_created_ts: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        ftm_provenance: json!({"important": "information"}),
    }];
    crate::cloud::messages_req::create_thread_messages(
        gcx.clone(),
        &thread.ft_id,
        output_thread_messages,
    ).await?;
    Ok(())
}

async fn call_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
) -> Result<(), String> {
    let max_new_tokens = 8192;
    let last_message_num = thread_messages.iter().map(|x| x.ftm_num).max().unwrap_or(0);
    let (alt, prev_alt) = thread_messages
        .last()
        .map(|msg| (msg.ftm_alt, msg.ftm_prev_alt))
        .unwrap_or((0, 0));
    let messages = crate::cloud::messages_req::convert_thread_messages_to_messages(thread_messages);
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0)
        .await
        .map_err_to_string()?;
    let model_rec = crate::caps::resolve_chat_model(caps, &format!("refact/{}", thread.ft_model))
        .map_err(|e| format!("Failed to resolve chat model: {}", e))?;
    let ccx = Arc::new(AMutex::new(
        AtCommandsContext::new(
            gcx.clone(),
            model_rec.base.n_ctx,
            12,
            false,
            messages.clone(),
            thread.ft_id.to_string(),
            false,
            thread.ft_model.to_string(),
        ).await,
    ));
    let allowed_tools = crate::cloud::messages_req::get_tool_names_from_openai_format(&thread.ft_toolset).await?;
    let mut all_tools: IndexMap<String, Box<dyn crate::tools::tools_description::Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone()).await
            .into_iter()
            .filter(|x| allowed_tools.contains(&x.tool_description().name))
            .map(|x| (x.tool_description().name, x))
            .collect();
    let mut has_rag_results = crate::scratchpads::scratchpad_utils::HasRagResults::new();
    let tokenizer_arc = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await?;
    let messages_count = messages.len();
    let (output_messages, _) = crate::tools::tools_execute::run_tools_locally(
        ccx.clone(),
        &mut all_tools,
        tokenizer_arc,
        max_new_tokens,
        &messages,
        &mut has_rag_results,
        &None,
    ).await?;
    if messages.len() == output_messages.len() {
        tracing::warn!(
            "Thread has no active tool call awaiting but still has need_tool_call turned on"
        );
        return Ok(());
    }
    let output_thread_messages = crate::cloud::messages_req::convert_messages_to_thread_messages(
        output_messages.into_iter().skip(messages_count).collect(),
        alt,
        prev_alt,
        last_message_num + 1,
        &thread.ft_id,
    )?;
    crate::cloud::messages_req::create_thread_messages(
        gcx.clone(),
        &thread.ft_id,
        output_thread_messages,
    ).await?;
    Ok(())
}
