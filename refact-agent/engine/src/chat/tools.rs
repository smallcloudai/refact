use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::info;
use uuid::Uuid;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatMode};
use crate::global_context::GlobalContext;
use crate::constants::CHAT_TOP_N;

use super::types::*;
use super::generation::start_generation;
use super::trajectories::maybe_save_trajectory;

fn is_server_executed_tool(tool_call_id: &str) -> bool {
    tool_call_id.starts_with("srvtoolu_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_server_executed_tool_with_prefix() {
        assert!(is_server_executed_tool("srvtoolu_abc123"));
        assert!(is_server_executed_tool("srvtoolu_"));
        assert!(is_server_executed_tool("srvtoolu_very_long_id_here"));
    }

    #[test]
    fn test_is_server_executed_tool_without_prefix() {
        assert!(!is_server_executed_tool("call_abc123"));
        assert!(!is_server_executed_tool("toolu_abc123"));
        assert!(!is_server_executed_tool(""));
        assert!(!is_server_executed_tool("srvtoolu"));
        assert!(!is_server_executed_tool("SRVTOOLU_abc"));
    }
}

pub async fn check_tool_calls_and_continue(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
    chat_mode: ChatMode,
) {
    let (tool_calls, messages, thread) = {
        let session = session_arc.lock().await;
        let last_msg = session.messages.last();
        match last_msg {
            Some(m) if m.role == "assistant" && m.tool_calls.is_some() => {
                let all_calls = m.tool_calls.clone().unwrap();
                let client_calls: Vec<_> = all_calls.into_iter()
                    .filter(|tc| !is_server_executed_tool(&tc.id))
                    .collect();
                (
                    client_calls,
                    session.messages.clone(),
                    session.thread.clone(),
                )
            }
            _ => {
                session.queue_notify.notify_one();
                return;
            }
        }
    };

    if tool_calls.is_empty() {
        let session = session_arc.lock().await;
        session.queue_notify.notify_one();
        return;
    }

    info!("check_tool_calls_and_continue: {} tool calls to process", tool_calls.len());

    let (confirmations, denials) = check_tools_confirmation(gcx.clone(), &tool_calls, &messages, chat_mode).await;

    let denied_ids: Vec<String> = denials.iter().map(|d| d.tool_call_id.clone()).collect();
    if !denials.is_empty() {
        let mut session = session_arc.lock().await;
        for denial in &denials {
            let tool_message = ChatMessage {
                message_id: Uuid::new_v4().to_string(),
                role: "tool".to_string(),
                content: ChatContent::SimpleText(format!("Denied by policy: {}", denial.rule)),
                tool_call_id: denial.tool_call_id.clone(),
                tool_failed: Some(true),
                ..Default::default()
            };
            session.add_message(tool_message);
        }
    }

    if !confirmations.is_empty() {
        let mut session = session_arc.lock().await;
        session.set_paused_with_reasons(confirmations);
        return;
    }

    let tools_to_execute: Vec<_> = tool_calls.iter()
        .filter(|tc| !denied_ids.contains(&tc.id))
        .cloned()
        .collect();

    if tools_to_execute.is_empty() {
        start_generation(gcx, session_arc).await;
        return;
    }

    {
        let mut session = session_arc.lock().await;
        session.set_runtime_state(SessionState::ExecutingTools, None);
    }

    let tool_results = execute_tools(gcx.clone(), &tools_to_execute, &messages, &thread, chat_mode).await;

    {
        let mut session = session_arc.lock().await;
        for result_msg in tool_results {
            session.add_message(result_msg);
        }
        session.set_runtime_state(SessionState::Idle, None);
    }

    maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
    start_generation(gcx, session_arc).await;
}

pub async fn check_tools_confirmation(
    gcx: Arc<ARwLock<GlobalContext>>,
    tool_calls: &[crate::call_validation::ChatToolCall],
    messages: &[ChatMessage],
    chat_mode: ChatMode,
) -> (Vec<PauseReason>, Vec<PauseReason>) {
    use crate::tools::tools_description::MatchConfirmDenyResult;

    let mut confirmations = Vec::new();
    let mut denials = Vec::new();

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        1000,
        1,
        false,
        messages.to_vec(),
        String::new(),
        false,
        String::new(),
    ).await));

    let all_tools = crate::tools::tools_list::get_available_tools_by_chat_mode(gcx.clone(), chat_mode).await
        .into_iter()
        .map(|tool| {
            let spec = tool.tool_description();
            (spec.name, tool)
        })
        .collect::<indexmap::IndexMap<_, _>>();

    for tool_call in tool_calls {
        let tool = match all_tools.get(&tool_call.function.name) {
            Some(t) => t,
            None => {
                info!("Unknown tool: {}, skipping confirmation check", tool_call.function.name);
                continue;
            }
        };

        let args: std::collections::HashMap<String, serde_json::Value> =
            match serde_json::from_str(&tool_call.function.arguments) {
                Ok(a) => a,
                Err(e) => {
                    denials.push(PauseReason {
                        reason_type: "denial".to_string(),
                        command: tool_call.function.name.clone(),
                        rule: format!("Failed to parse arguments: {}", e),
                        tool_call_id: tool_call.id.clone(),
                        integr_config_path: tool.has_config_path(),
                    });
                    continue;
                }
            };

        match tool.match_against_confirm_deny(ccx.clone(), &args).await {
            Ok(result) => {
                match result.result {
                    MatchConfirmDenyResult::DENY => {
                        denials.push(PauseReason {
                            reason_type: "denial".to_string(),
                            command: result.command,
                            rule: result.rule,
                            tool_call_id: tool_call.id.clone(),
                            integr_config_path: tool.has_config_path(),
                        });
                    }
                    MatchConfirmDenyResult::CONFIRMATION => {
                        confirmations.push(PauseReason {
                            reason_type: "confirmation".to_string(),
                            command: result.command,
                            rule: result.rule,
                            tool_call_id: tool_call.id.clone(),
                            integr_config_path: tool.has_config_path(),
                        });
                    }
                    _ => {}
                }
            }
            Err(e) => {
                info!("Error checking confirmation for {}: {}", tool_call.function.name, e);
            }
        }
    }

    (confirmations, denials)
}

pub async fn execute_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    tool_calls: &[crate::call_validation::ChatToolCall],
    messages: &[ChatMessage],
    thread: &ThreadParams,
    chat_mode: ChatMode,
) -> Vec<ChatMessage> {
    let mut result_messages = Vec::new();

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        thread.context_tokens_cap.unwrap_or(8192),
        CHAT_TOP_N,
        false,
        messages.to_vec(),
        thread.id.clone(),
        false,
        thread.model.clone(),
    ).await));

    let mut all_tools = crate::tools::tools_list::get_available_tools_by_chat_mode(gcx.clone(), chat_mode).await
        .into_iter()
        .map(|tool| {
            let spec = tool.tool_description();
            (spec.name, tool)
        })
        .collect::<indexmap::IndexMap<_, _>>();

    for tool_call in tool_calls {
        let tool = match all_tools.get_mut(&tool_call.function.name) {
            Some(t) => t,
            None => {
                result_messages.push(ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(
                        format!("Error: tool '{}' not found", tool_call.function.name)
                    ),
                    tool_call_id: tool_call.id.clone(),
                    tool_failed: Some(true),
                    ..Default::default()
                });
                continue;
            }
        };

        let args: std::collections::HashMap<String, serde_json::Value> =
            match serde_json::from_str(&tool_call.function.arguments) {
                Ok(a) => a,
                Err(e) => {
                    result_messages.push(ChatMessage {
                        message_id: Uuid::new_v4().to_string(),
                        role: "tool".to_string(),
                        content: ChatContent::SimpleText(
                            format!("Error parsing arguments: {}", e)
                        ),
                        tool_call_id: tool_call.id.clone(),
                        tool_failed: Some(true),
                        ..Default::default()
                    });
                    continue;
                }
            };

        info!("Executing tool: {}({:?})", tool_call.function.name, args);

        match tool.tool_execute(ccx.clone(), &tool_call.id, &args).await {
            Ok((_corrections, results)) => {
                let mut context_files: Vec<crate::call_validation::ContextFile> = Vec::new();

                for result in results {
                    match result {
                        crate::call_validation::ContextEnum::ChatMessage(mut msg) => {
                            if msg.message_id.is_empty() {
                                msg.message_id = Uuid::new_v4().to_string();
                            }
                            if msg.tool_failed.is_none() {
                                msg.tool_failed = Some(false);
                            }
                            result_messages.push(msg);
                        }
                        crate::call_validation::ContextEnum::ContextFile(cf) => {
                            context_files.push(cf);
                        }
                    }
                }

                if !context_files.is_empty() {
                    result_messages.push(ChatMessage {
                        message_id: Uuid::new_v4().to_string(),
                        role: "context_file".to_string(),
                        content: ChatContent::ContextFiles(context_files),
                        ..Default::default()
                    });
                }
            }
            Err(e) => {
                info!("Tool execution failed: {}: {}", tool_call.function.name, e);
                result_messages.push(ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(format!("Error: {}", e)),
                    tool_call_id: tool_call.id.clone(),
                    tool_failed: Some(true),
                    ..Default::default()
                });
            }
        }
    }

    result_messages
}
