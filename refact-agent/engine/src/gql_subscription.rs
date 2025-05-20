use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock as ARwLock;
use tracing::{error, info};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use url::Url;

use crate::global_context::GlobalContext;

const GRAPHQL_WS_URL: &str = "ws://localhost:8008/v1/superuser";
const API_KEY: &str = "sk_alice_123456";

pub async fn watch_threads_subscription(
    global_context: Arc<ARwLock<GlobalContext>>,
) {
    info!("Starting GraphQL subscription for threads_to_advance");
    
    // Subscription query
    let subscription_query = r#"
    subscription WatchThreads {
        threads_to_advance {
            news_action
            news_payload_id
            news_payload {
                owner_fuser_id
                owner_shared
                fgroup_id
                ft_id
                ft_title
                ft_belongs_to_fce_id
                ft_model
                ft_temperature
                ft_max_new_tokens
                ft_n
                ft_need_assistant
                ft_error
                ft_created_ts
                ft_updated_ts
                ft_archived_ts
            }
        }
    }
    "#;

    loop {
        if global_context.read().await.shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Shutting down GraphQL subscription thread");
            break;
        }

        match connect_to_graphql_ws(subscription_query).await {
            Ok(()) => {
                // Connection closed normally, wait before reconnecting
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("Error in GraphQL subscription: {}", e);
                // Wait before retry
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn connect_to_graphql_ws(subscription_query: &str) -> Result<(), String> {
    // Parse the WebSocket URL
    let url = Url::parse(GRAPHQL_WS_URL)
        .map_err(|e| format!("Failed to parse WebSocket URL: {}", e))?;

    // Connect to the WebSocket server
    let (ws_stream, _) = connect_async(url)
        .await
        .map_err(|e| format!("Failed to connect to WebSocket server: {}", e))?;
    
    info!("Connected to GraphQL WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();
    
    // Send connection initialization message with API key
    let init_message = json!({
        "type": "connection_init",
        "payload": {
            "apikey": API_KEY
        }
    });
    
    write.send(Message::Text(init_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send connection init message: {}", e))?;
    
    // Wait for connection acknowledgment
    if let Some(msg) = read.next().await {
        let msg = msg.map_err(|e| format!("WebSocket error: {}", e))?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Failed to parse connection response: {}", e))?;
            
            if response["type"] != "connection_ack" {
                return Err(format!("Expected connection_ack, got: {}", response));
            }
            
            info!("GraphQL connection acknowledged");
        }
    }
    
    // Send subscription message
    let subscription_message = json!({
        "id": "1",
        "type": "start",
        "payload": {
            "query": subscription_query
        }
    });
    
    write.send(Message::Text(subscription_message.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription message: {}", e))?;
    
    info!("Subscription started, waiting for events...");
    
    // Process incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to parse message: {}, error: {}", text, e);
                        continue;
                    }
                };
                
                match response["type"].as_str() {
                    Some("data") => {
                        if let Some(data) = response["payload"]["data"]["threads_to_advance"].as_object() {
                            let action = data["news_action"].as_str().unwrap_or("unknown");
                            let payload_id = data["news_payload_id"].as_str().unwrap_or("unknown");
                            
                            info!("Received thread event: action={}, id={}", action, payload_id);
                            
                            if let Some(payload) = data["news_payload"].as_object() {
                                if let Some(need_assistant) = payload["ft_need_assistant"].as_i64() {
                                    if need_assistant >= 0 {
                                        info!("Thread {} needs assistance, alt={}", payload_id, need_assistant);
                                    }
                                }
                            }
                        }
                    }
                    Some("error") => {
                        error!("Subscription error: {}", response);
                    }
                    Some("complete") => {
                        info!("Subscription completed");
                        break;
                    }
                    _ => {
                        info!("Received message: {}", text);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Ok(_) => {} // Ignore other message types
            Err(e) => {
                return Err(format!("WebSocket error: {}", e));
            }
        }
    }
    
    Ok(())
}
