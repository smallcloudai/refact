use std::future::Future;
use crate::global_context::GlobalContext;
use futures::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::sync::RwLock as ARwLock;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;

const RECONNECT_DELAY_SECONDS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub owner_fuser_id: String,
    pub ft_id: String,
    pub ft_error: Option<Value>,
    pub ft_locked_by: String,
    pub ft_fexp_name: Option<String>,
    pub ft_need_tool_calls: i64,
    pub ft_need_user: i64,
    pub ft_app_searchable: Option<String>,
    pub ft_app_capture: Option<String>,
    pub ft_app_specific: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
          ft_fexp_name
          ft_need_tool_calls
          ft_need_user
          ft_app_searchable
          ft_app_capture
          ft_app_specific
        }
      }
    }
"#;


pub fn generate_random_hash(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}


pub async fn trigger_threads_subscription_restart(gcx: Arc<ARwLock<GlobalContext>>) {
    let restart_flag = gcx.read().await.threads_subscription_restart_flag.clone();
    restart_flag.store(true, Ordering::SeqCst);
    info!("threads subscription restart triggered");
}

pub async fn watch_threads_subscription(gcx: Arc<ARwLock<GlobalContext>>) {
    if !gcx.read().await.cmdline.cloud_threads {
        return;
    }
    // let api_key = gcx.read().await.cmdline.api_key.clone();
    // TODO: remove it later
    let api_key = "sk_alice_123456".to_string();
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
        let connection_result = initialize_connection(api_key.clone(), &located_fgroup_id).await;
        let mut connection = match connection_result {
            Ok(conn) => conn,
            Err(err) => {
                error!("failed to initialize connection: {}", err);
                info!("will attempt to reconnect in {} seconds", RECONNECT_DELAY_SECONDS);
                tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
                continue;
            }
        };

        let events_result = events_loop(
            gcx.clone(),
            &mut connection,
            api_key.clone(),
            |gcx, payload, basic_info, api_key, app_searchable_id| {
                let payload_owned = payload.clone();
                let basic_info_owned = basic_info.clone();
                async move {
                    match crate::cloud::threads_processing::process_thread_event(
                        gcx, &payload_owned, &basic_info_owned, api_key, app_searchable_id,
                    ).await {
                        Ok(_) => Ok(false),
                        Err(e) => Err(e)
                    }
                }
            },
        ).await;
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

pub async fn initialize_connection(
    api_key: String,
    located_fgroup_id: &str,
) -> Result<
    futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
    >,
    String,
> {
    let url = Url::parse(crate::constants::GRAPHQL_WS_URL)
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
                    if msg_type == "connection_ack" {} else if msg_type == "connection_error" {
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
    let id = generate_random_hash(16);
    let subscription_message = json!({
        "id": id,
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

pub async fn events_loop<F, Fut>(
    gcx: Arc<ARwLock<GlobalContext>>,
    connection: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    api_key: String,
    processor: F,
) -> Result<(), String>
where
    F: Fn(Arc<ARwLock<GlobalContext>>, &ThreadPayload, &BasicStuff, String, String) -> Fut + Send + Sync,
    Fut: Future<Output=Result<bool, String>> + Send,
{
    info!("cloud threads subscription started, waiting for events...");
    let app_searchable_id = gcx.read().await.app_searchable_id.clone();
    let basic_info = get_basic_info(api_key.clone()).await?;
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
                                match processor(
                                    gcx.clone(), &payload, &basic_info, api_key.clone(), app_searchable_id.clone(),
                                ).await {
                                    Ok(need_to_stop) => {
                                        if need_to_stop {
                                            info!("stopping threads subscription");
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        error!("failed to process thread event: {}\n{:?}", err, threads_in_group);
                                    }
                                }
                            } else {
                                info!("failed to parse thread payload: {:?}", threads_in_group);
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

pub async fn get_basic_info(api_key: String) -> Result<BasicStuff, String> {
    let client = Client::new();
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
        .post(&crate::constants::GRAPHQL_URL.to_string())
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
