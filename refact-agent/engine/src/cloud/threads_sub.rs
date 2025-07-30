use crate::global_context::GlobalContext;
use futures::{SinkExt, StreamExt};
use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig, graphql_error_to_string};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::RwLock as ARwLock;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;
use crate::basic_utils::generate_random_hash;

const RECONNECT_DELAY_SECONDS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub owner_fuser_id: String,
    pub ft_id: String,
    pub ft_error: Option<Value>,
    pub ft_locked_by: String,
    pub ft_fexp_id: Option<String>,
    pub ft_need_tool_calls: i64,
    pub ft_need_user: i64,
    pub ft_app_searchable: Option<String>,
    pub ft_app_capture: Option<String>,
    pub ft_app_specific: Option<Value>,
    pub ft_confirmation_request: Option<Value>,
    pub ft_confirmation_response: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicStuff {
    pub fuser_id: String,
    pub workspaces: Vec<Value>,
}

const THREADS_SUBSCRIPTION_QUERY: &str = r#"
    subscription ThreadsPageSubs($located_fgroup_id: String!, $filter: [String!]) {
      threads_in_group(located_fgroup_id: $located_fgroup_id, filter: $filter) {
        news_action
        news_payload_id
        news_payload {
          owner_fuser_id
          ft_id
          ft_error
          ft_locked_by
          ft_fexp_id
          ft_confirmation_request
          ft_confirmation_response
          ft_need_tool_calls
          ft_need_user
          ft_app_searchable
          ft_app_capture
          ft_app_specific
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
    let (address_url, api_key) = {
        let gcx_read = gcx.read().await;
        (gcx_read.cmdline.address_url.clone(), gcx_read.cmdline.api_key.clone())
    };
    loop {
        {
            let restart_flag = gcx.read().await.threads_subscription_restart_flag.clone();
            restart_flag.store(false, Ordering::SeqCst);
        }
        let (located_fgroup_id, app_searchable_id) = {
            let gcx_locked = gcx.read().await;
            let located_fgroup_id = if let Some(located_fgroup_id) = gcx_locked.active_group_id.clone() {
                located_fgroup_id
            } else {
                warn!("no active group is set, skipping threads subscription");
                tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
                continue;
            };
            (located_fgroup_id, gcx_locked.app_searchable_id.clone())
        };

        info!(
            "starting subscription for threads_in_group with fgroup_id=\"{}\" and app_searchable_id=\"{}\"",
            located_fgroup_id, app_searchable_id
        );
        let connection_result = initialize_connection(&address_url, &api_key, &located_fgroup_id, &app_searchable_id).await;
        let mut connection = match connection_result {
            Ok(conn) => conn,
            Err(err) => {
                error!("failed to initialize connection: {}", err);
                info!("will attempt to reconnect in {} seconds", RECONNECT_DELAY_SECONDS);
                tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECONDS)).await;
                continue;
            }
        };

        let events_result = actual_subscription_loop(
            gcx.clone(),
            &mut connection,
            &address_url,
            &api_key,
            &located_fgroup_id
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
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str,
    app_searchable_id: &str,
) -> Result<
    futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
    >,
    String,
> {
    let url = Url::parse(&crate::constants::get_graphql_ws_url(cmd_address_url))
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
    let subscription_message = json!({
        "id": generate_random_hash(16),
        "type": "start",
        "payload": {
            "query": THREADS_SUBSCRIPTION_QUERY,
            "variables": {
                "located_fgroup_id": located_fgroup_id,
                "filter": [format!("ft_app_searchable:eq:{}", app_searchable_id)]
            }
        }
    });
    write
        .send(Message::Text(subscription_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription message: {}", e))?;
    Ok(read)
}

async fn actual_subscription_loop(
    gcx: Arc<ARwLock<GlobalContext>>,
    connection: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str,
) -> Result<(), String> {
    info!("cloud threads subscription started, waiting for events...");
    let basic_info = get_basic_info(cmd_address_url, api_key).await?;
    while let Some(msg) = connection.next().await {
        if gcx.clone().read().await.shutdown_flag.load(Ordering::SeqCst) {
            info!("shutting down threads subscription");
            break;
        }
        if gcx.clone().read().await.threads_subscription_restart_flag.load(Ordering::SeqCst) {
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
                match response["type"].as_str().unwrap_or("unknown") {
                    "data" => {
                        if let Some(payload) = response["payload"].as_object() {
                            let data = &payload["data"];
                            let threads_in_group = &data["threads_in_group"];
                            let news_action = threads_in_group["news_action"].as_str().unwrap_or("");
                            if news_action != "INSERT" && news_action != "UPDATE" {
                                continue;
                            }
                            if let Ok(payload) = serde_json::from_value::<ThreadPayload>(threads_in_group["news_payload"].clone()) {
                                let gcx_clone = gcx.clone();
                                let payload_clone = payload.clone();
                                let basic_info_clone = basic_info.clone();
                                let cmd_address_url_clone = cmd_address_url.to_string();
                                let api_key_clone = api_key.to_string();
                                let located_fgroup_id_clone = located_fgroup_id.to_string();
                                tokio::spawn(async move {
                                    crate::cloud::threads_processing::process_thread_event(
                                        gcx_clone, payload_clone, basic_info_clone, cmd_address_url_clone, api_key_clone, located_fgroup_id_clone
                                    ).await
                                });
                            } else {
                                info!("failed to parse thread payload: {:?}", threads_in_group);
                            }
                        } else {
                            info!("received data message but couldn't find payload");
                        }
                    }
                    "error" => {
                        error!("threads subscription error: {}", text);
                        return Err(format!("{}", text));
                    }
                    "complete" => {
                        error!("threads subscription complete: {}.\nRestarting it", text);
                        return Err(format!("{}", text));
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

pub async fn get_basic_info(cmd_address_url: &str, api_key: &str) -> Result<BasicStuff, String> {
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

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        user_agent: Some("refact-lsp".to_string()),
        additional_headers: None,
    };

    execute_graphql::<BasicStuff, _>(
        config,
        query,
        json!({}),
        "query_basic_stuff"
    )
    .await
    .map_err(graphql_error_to_string)
}
