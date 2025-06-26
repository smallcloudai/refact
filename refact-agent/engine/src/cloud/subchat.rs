use std::sync::Arc;
use std::sync::atomic::Ordering;
use futures::StreamExt;
use crate::at_commands::at_commands::AtCommandsContext;
use tokio::sync::Mutex as AMutex;
use crate::call_validation::{ChatMessage, ReasoningEffort};
use crate::cloud::{threads_req, messages_req};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};
use crate::cloud::threads_sub::{initialize_connection, ThreadPayload};

#[derive(Serialize, Deserialize, Debug)]
struct KernelOutput {
    pub logs: Vec<String>,
    pub detail: String,
    pub flagged_by_kernel: bool
}

fn build_preferences(
    model: &str,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    n: usize,
    reasoning_effort: Option<ReasoningEffort>,
) -> serde_json::Value {
    let mut preferences = serde_json::json!({
        "model": model,
        "n": n,
    });
    if let Some(temp) = temperature {
        preferences["temperature"] = serde_json::json!(temp);
    }
    if let Some(max_tokens) = max_new_tokens {
        preferences["max_new_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(reasoning_effort) = reasoning_effort {
        preferences["reasoning_effort"] = serde_json::json!(reasoning_effort);
    }
    preferences
}

pub async fn subchat(
    ccx: Arc<AMutex<AtCommandsContext>>,
    ft_fexp_id: &str,
    tool_call_id: &str,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_new_tokens: Option<usize>,
    reasoning_effort: Option<ReasoningEffort>,
) -> Result<Vec<ChatMessage>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let (cmd_address_url, api_key, app_searchable_id, located_fgroup_id, parent_thread_id) = {
        let gcx_read = gcx.read().await;
        let located_fgroup_id = gcx_read.active_group_id.clone()
            .ok_or("No active group ID is set".to_string())?;
        (
            gcx_read.cmdline.address_url.clone(),
            gcx_read.cmdline.api_key.clone(),
            gcx_read.app_searchable_id.clone(),
            located_fgroup_id,
            ccx.lock().await.chat_id.clone(),
        )
    };
    
    let model_name = crate::cloud::experts_req::expert_choice_consequences(&cmd_address_url, &api_key, ft_fexp_id, &located_fgroup_id).await?;
    let preferences = build_preferences(&model_name, temperature, max_new_tokens, 1, reasoning_effort);
    let existing_threads = crate::cloud::threads_req::get_threads_app_captured(
        &cmd_address_url,
        &api_key,
        &located_fgroup_id,
        &app_searchable_id,
        tool_call_id
    ).await?;
    let thread = if !existing_threads.is_empty() {
        info!("There are already existing threads for this tool_id: {:?}", existing_threads);
        existing_threads[0].clone()
    } else {
        let thread = threads_req::create_thread(
            &cmd_address_url,
            &api_key,
            &located_fgroup_id,
            ft_fexp_id,
            &format!("subchat_{}", ft_fexp_id),
            &tool_call_id,
            &app_searchable_id,
            serde_json::json!({
            "tool_call_id": tool_call_id,
            "ft_fexp_id": ft_fexp_id,
        }),
            None,
            Some(parent_thread_id)
        ).await?;
        let thread_messages = messages_req::convert_messages_to_thread_messages(
            messages, 100, 100, 1, &thread.ft_id, Some(preferences)
        )?;
        messages_req::create_thread_messages(
            &cmd_address_url, &api_key, &thread.ft_id, thread_messages
        ).await?;
        thread
    };
    
    let thread_id = thread.ft_id.clone();
    let connection_result = initialize_connection(&cmd_address_url, &api_key, &located_fgroup_id).await;
    let mut connection = match connection_result {
        Ok(conn) => conn,
        Err(err) => return Err(format!("Failed to initialize WebSocket connection: {}", err)),
    };
    while let Some(msg) = connection.next().await {
        if gcx.read().await.shutdown_flag.load(Ordering::SeqCst) {
            info!("shutting down threads subscription");
            break;
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
                                if payload.ft_id != thread_id {
                                    continue;
                                }
                                if payload.ft_error.is_some() {
                                    break;
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
                    "complete" => {
                        error!("threads subscription complete: {}.\nRestarting it", text);
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

    let thread = threads_req::get_thread(&cmd_address_url, &api_key, &thread_id).await?;
    if let Some(error) = thread.ft_error {
        // the error might be actually a kernel output data
        if let Some(kernel_output) = serde_json::from_str::<KernelOutput>(&error.to_string()).ok() {
            info!("subchat was terminated by kernel: {:?}", kernel_output);
        } else {
            return Err(format!("Thread error: {:?}", error));
        }
    }
    
    let all_thread_messages = messages_req::get_thread_messages(
        &cmd_address_url, &api_key, &thread_id, 100
    ).await?;
    Ok(messages_req::convert_thread_messages_to_messages(&all_thread_messages)
        .into_iter()
        .filter(|x| x.role != "kernel")
        .collect::<Vec<_>>())
}
