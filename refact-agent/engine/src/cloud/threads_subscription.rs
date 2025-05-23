use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock as ARwLock;
use tracing::{error, info, warn};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async, 
    tungstenite::{
        protocol::Message,
    }
};
use serde_json::{json, Value};
use url::Url;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use serde::{Deserialize, Serialize};
use crate::cloud::threads_req::{Thread, ThreadMessage};
use crate::global_context::GlobalContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub owner_fuser_id: String,
    pub owner_shared: bool,
    pub ft_id: String,
    pub ft_fexp_name: Option<String>,
    pub ft_fexp_ver_major: Option<i64>,
    pub ft_fexp_ver_minor: Option<i64>,
    pub ft_title: String,
    pub ft_error: String,
    pub ft_updated_ts: f64,
    pub ft_locked_by: String,
    pub ft_need_assistant: i64,
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

async fn events_loop(
    gcx: Arc<ARwLock<GlobalContext>>,
    connection: &mut futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>
) -> Result<(), String> {
    info!("Cloud threads subscription started, waiting for events...");
    
    let shutdown_flag = &gcx.read().await.shutdown_flag;
    while let Some(msg) = connection.next().await {
        if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
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
                        if let Some(payload) = response.payload {
                            let thread_event = &payload.data.threads_in_group;
                            let payload_id = &thread_event.news_payload_id;
                            match &thread_event.news_action {
                                NewsAction::Insert | NewsAction::Update => {
                                    info!("Thread was {}: id={}", thread_event.news_action.to_lowercase(), payload_id);
                                    

                                    if let Some(payload) = &thread_event.news_payload {
                                        match crate::cloud::threads_req::get_thread(gcx.clone(), &payload.ft_id).await {
                                            Ok(t) => {
                                                warn!("Thread:\n{:?}", t);
                                            },
                                            Err(err) => {
                                                error!("{}", err);
                                            }
                                        };
                                        
                                        match crate::cloud::threads_req::get_thread_messages(gcx.clone(), &payload.ft_id, 100).await {
                                            Ok(messages) => {
                                                warn!("Thread messages:\n{:?}", messages);
                                            },
                                            Err(err) => {
                                                error!("{}", err);
                                            }
                                        }
                                        
                                        match crate::cloud::experts_req::get_expert(gcx.clone(), "agent").await {
                                            Ok(messages) => {
                                                warn!("Expert:\n{:?}", messages);
                                            },
                                            Err(err) => {
                                                error!("{}", err);
                                            }
                                        }
                                    }
                                },
                                NewsAction::InitialUpdatesOver => {
                                    info!("Initial thread updates completed - subscription is now live");
                                },
                                _ => {}
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
