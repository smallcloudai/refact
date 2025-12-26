use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::warn;
use uuid::Uuid;

use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::GlobalContext;

use super::types::*;
use super::content::parse_content_with_attachments;
use super::generation::start_generation;
use super::tools::execute_tools;
use super::trajectories::maybe_save_trajectory;

pub fn find_allowed_command_while_paused(queue: &VecDeque<CommandRequest>) -> Option<usize> {
    for (i, req) in queue.iter().enumerate() {
        match &req.command {
            ChatCommand::ToolDecision { .. }
            | ChatCommand::ToolDecisions { .. }
            | ChatCommand::Abort {} => {
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

pub fn apply_setparams_patch(thread: &mut ThreadParams, patch: &serde_json::Value) -> (bool, serde_json::Value) {
    let mut changed = false;

    if let Some(model) = patch.get("model").and_then(|v| v.as_str()) {
        if thread.model != model {
            thread.model = model.to_string();
            changed = true;
        }
    }
    if let Some(mode) = patch.get("mode").and_then(|v| v.as_str()) {
        if thread.mode != mode {
            thread.mode = mode.to_string();
            changed = true;
        }
    }
    if let Some(boost) = patch.get("boost_reasoning").and_then(|v| v.as_bool()) {
        if thread.boost_reasoning != boost {
            thread.boost_reasoning = boost;
            changed = true;
        }
    }
    if let Some(tool_use) = patch.get("tool_use").and_then(|v| v.as_str()) {
        if thread.tool_use != tool_use {
            thread.tool_use = tool_use.to_string();
            changed = true;
        }
    }
    if let Some(cap) = patch.get("context_tokens_cap") {
        let new_cap = cap.as_u64().map(|n| n as usize);
        if thread.context_tokens_cap != new_cap {
            thread.context_tokens_cap = new_cap;
            changed = true;
        }
    }
    if let Some(include) = patch.get("include_project_info").and_then(|v| v.as_bool()) {
        if thread.include_project_info != include {
            thread.include_project_info = include;
            changed = true;
        }
    }
    if let Some(enabled) = patch.get("checkpoints_enabled").and_then(|v| v.as_bool()) {
        if thread.checkpoints_enabled != enabled {
            thread.checkpoints_enabled = enabled;
            changed = true;
        }
    }
    if let Some(compression) = patch.get("use_compression").and_then(|v| v.as_bool()) {
        if thread.use_compression != compression {
            thread.use_compression = compression;
            changed = true;
        }
    }

    let mut sanitized_patch = patch.clone();
    if let Some(obj) = sanitized_patch.as_object_mut() {
        obj.remove("type");
        obj.remove("chat_id");
        obj.remove("seq");
    }

    (changed, sanitized_patch)
}

pub async fn process_command_queue(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
    processor_running: Arc<AtomicBool>,
) {
    struct ProcessorGuard(Arc<AtomicBool>);
    impl Drop for ProcessorGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = ProcessorGuard(processor_running);

    loop {
        let command = {
            let mut session = session_arc.lock().await;

            if session.closed {
                return;
            }

            let state = session.runtime.state;
            let is_busy = state == SessionState::Generating
                || state == SessionState::ExecutingTools
                || state == SessionState::WaitingIde;

            if is_busy {
                let notify = session.queue_notify.clone();
                let waiter = notify.notified();
                drop(session);
                waiter.await;
                continue;
            }

            if state == SessionState::Paused {
                if let Some(idx) = find_allowed_command_while_paused(&session.command_queue) {
                    session.command_queue.remove(idx)
                } else {
                    let notify = session.queue_notify.clone();
                    let waiter = notify.notified();
                    drop(session);
                    waiter.await;
                    continue;
                }
            } else if session.command_queue.is_empty() {
                let notify = session.queue_notify.clone();
                let closed = session.closed;
                drop(session);

                if closed {
                    return;
                }

                maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;

                let session = session_arc.lock().await;
                if session.closed {
                    return;
                }
                if session.command_queue.is_empty() {
                    let waiter = notify.notified();
                    drop(session);
                    waiter.await;
                    continue;
                }
                drop(session);
                continue;
            } else {
                session.command_queue.pop_front()
            }
        };

        let Some(request) = command else {
            continue;
        };

        match request.command {
            ChatCommand::UserMessage { content, attachments } => {
                let mut session = session_arc.lock().await;
                let parsed_content = parse_content_with_attachments(&content, &attachments);

                let checkpoints = if session.thread.checkpoints_enabled {
                    create_checkpoint_for_message(gcx.clone(), &session).await
                } else {
                    Vec::new()
                };

                let user_message = ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "user".to_string(),
                    content: parsed_content,
                    checkpoints,
                    ..Default::default()
                };
                session.add_message(user_message);
                drop(session);

                maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
                start_generation(gcx.clone(), session_arc.clone()).await;
            }
            ChatCommand::RetryFromIndex { index, content, attachments } => {
                let mut session = session_arc.lock().await;
                session.truncate_messages(index);
                let parsed_content = parse_content_with_attachments(&content, &attachments);
                let user_message = ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "user".to_string(),
                    content: parsed_content,
                    ..Default::default()
                };
                session.add_message(user_message);
                drop(session);

                maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
                start_generation(gcx.clone(), session_arc.clone()).await;
            }
            ChatCommand::SetParams { patch } => {
                if !patch.is_object() {
                    warn!("SetParams patch must be an object, ignoring");
                    continue;
                }
                let mut session = session_arc.lock().await;
                let (mut changed, sanitized_patch) = apply_setparams_patch(&mut session.thread, &patch);

                let title_in_patch = patch.get("title").and_then(|v| v.as_str());
                let is_gen_in_patch = patch.get("is_title_generated").and_then(|v| v.as_bool());
                if let Some(title) = title_in_patch {
                    let is_generated = is_gen_in_patch.unwrap_or(false);
                    session.set_title(title.to_string(), is_generated);
                } else if let Some(is_gen) = is_gen_in_patch {
                    if session.thread.is_title_generated != is_gen {
                        session.thread.is_title_generated = is_gen;
                        let title = session.thread.title.clone();
                        session.emit(ChatEvent::TitleUpdated {
                            title,
                            is_generated: is_gen,
                        });
                        changed = true;
                    }
                }
                session.emit(ChatEvent::ThreadUpdated { params: sanitized_patch });
                if changed {
                    session.increment_version();
                    session.touch();
                }
            }
            ChatCommand::Abort {} => {
                let mut session = session_arc.lock().await;
                session.abort_stream();
            }
            ChatCommand::ToolDecision { tool_call_id, accepted } => {
                let decisions = vec![ToolDecisionItem { tool_call_id: tool_call_id.clone(), accepted }];
                handle_tool_decisions(gcx.clone(), session_arc.clone(), &decisions).await;
            }
            ChatCommand::ToolDecisions { decisions } => {
                handle_tool_decisions(gcx.clone(), session_arc.clone(), &decisions).await;
            }
            ChatCommand::IdeToolResult { tool_call_id, content, tool_failed } => {
                let mut session = session_arc.lock().await;
                let tool_message = ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText(content),
                    tool_call_id,
                    tool_failed: Some(tool_failed),
                    ..Default::default()
                };
                session.add_message(tool_message);
                session.set_runtime_state(SessionState::Idle, None);
                drop(session);
                start_generation(gcx.clone(), session_arc.clone()).await;
            }
            ChatCommand::UpdateMessage { message_id, content, attachments, regenerate } => {
                let mut session = session_arc.lock().await;
                if session.runtime.state == SessionState::Generating {
                    session.abort_stream();
                }
                let parsed_content = parse_content_with_attachments(&content, &attachments);
                if let Some(idx) = session.messages.iter().position(|m| m.message_id == message_id) {
                    let mut updated_msg = session.messages[idx].clone();
                    updated_msg.content = parsed_content;
                    session.update_message(&message_id, updated_msg);
                    if regenerate && idx + 1 < session.messages.len() {
                        session.truncate_messages(idx + 1);
                        drop(session);
                        maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
                        start_generation(gcx.clone(), session_arc.clone()).await;
                    }
                }
            }
            ChatCommand::RemoveMessage { message_id, regenerate } => {
                let mut session = session_arc.lock().await;
                if session.runtime.state == SessionState::Generating {
                    session.abort_stream();
                }
                if let Some(idx) = session.remove_message(&message_id) {
                    if regenerate && idx < session.messages.len() {
                        session.truncate_messages(idx);
                        drop(session);
                        maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
                        start_generation(gcx.clone(), session_arc.clone()).await;
                    }
                }
            }
        }
    }
}

async fn handle_tool_decisions(
    gcx: Arc<ARwLock<GlobalContext>>,
    session_arc: Arc<AMutex<ChatSession>>,
    decisions: &[ToolDecisionItem],
) {
    let (accepted_ids, has_remaining_pauses, tool_calls_to_execute, messages, thread) = {
        let mut session = session_arc.lock().await;
        let accepted = session.process_tool_decisions(decisions);
        let remaining = !session.runtime.pause_reasons.is_empty();

        for decision in decisions {
            if !decision.accepted {
                let tool_message = ChatMessage {
                    message_id: Uuid::new_v4().to_string(),
                    role: "tool".to_string(),
                    content: ChatContent::SimpleText("Tool execution denied by user".to_string()),
                    tool_call_id: decision.tool_call_id.clone(),
                    tool_failed: Some(true),
                    ..Default::default()
                };
                session.add_message(tool_message);
            }
        }

        let tool_calls: Vec<crate::call_validation::ChatToolCall> = session.messages.iter()
            .filter_map(|m| m.tool_calls.as_ref())
            .flatten()
            .filter(|tc| accepted.contains(&tc.id))
            .cloned()
            .collect();

        (accepted, remaining, tool_calls, session.messages.clone(), session.thread.clone())
    };

    if has_remaining_pauses {
        return;
    }

    if !accepted_ids.is_empty() && !tool_calls_to_execute.is_empty() {
        {
            let mut session = session_arc.lock().await;
            session.set_runtime_state(SessionState::ExecutingTools, None);
        }

        let chat_mode = super::generation::parse_chat_mode(&thread.mode);
        let (tool_results, _) = execute_tools(gcx.clone(), &tool_calls_to_execute, &messages, &thread, chat_mode, super::tools::ExecuteToolsOptions::default()).await;

        {
            let mut session = session_arc.lock().await;
            for result_msg in tool_results {
                session.add_message(result_msg);
            }
            session.set_runtime_state(SessionState::Idle, None);
        }

        maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
    }

    start_generation(gcx, session_arc).await;
}

async fn create_checkpoint_for_message(
    gcx: Arc<ARwLock<GlobalContext>>,
    session: &ChatSession,
) -> Vec<crate::git::checkpoints::Checkpoint> {
    use crate::git::checkpoints::create_workspace_checkpoint;

    let latest_checkpoint = session.messages.iter().rev()
        .find(|msg| msg.role == "user" && !msg.checkpoints.is_empty())
        .and_then(|msg| msg.checkpoints.first().cloned());

    match create_workspace_checkpoint(gcx, latest_checkpoint.as_ref(), &session.chat_id).await {
        Ok((checkpoint, _)) => {
            tracing::info!("Checkpoint created for chat {}: {:?}", session.chat_id, checkpoint);
            vec![checkpoint]
        }
        Err(e) => {
            warn!("Failed to create checkpoint for chat {}: {}", session.chat_id, e);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_request(cmd: ChatCommand) -> CommandRequest {
        CommandRequest {
            client_request_id: "req-1".into(),
            command: cmd,
        }
    }

    #[test]
    fn test_find_allowed_command_empty_queue() {
        let queue = VecDeque::new();
        assert!(find_allowed_command_while_paused(&queue).is_none());
    }

    #[test]
    fn test_find_allowed_command_no_allowed() {
        let mut queue = VecDeque::new();
        queue.push_back(make_request(ChatCommand::UserMessage {
            content: json!("hi"),
            attachments: vec![],
        }));
        queue.push_back(make_request(ChatCommand::SetParams {
            patch: json!({"model": "gpt-4"}),
        }));
        assert!(find_allowed_command_while_paused(&queue).is_none());
    }

    #[test]
    fn test_find_allowed_command_finds_tool_decision() {
        let mut queue = VecDeque::new();
        queue.push_back(make_request(ChatCommand::UserMessage {
            content: json!("hi"),
            attachments: vec![],
        }));
        queue.push_back(make_request(ChatCommand::ToolDecision {
            tool_call_id: "tc1".into(),
            accepted: true,
        }));
        assert_eq!(find_allowed_command_while_paused(&queue), Some(1));
    }

    #[test]
    fn test_find_allowed_command_finds_tool_decisions() {
        let mut queue = VecDeque::new();
        queue.push_back(make_request(ChatCommand::ToolDecisions {
            decisions: vec![ToolDecisionItem { tool_call_id: "tc1".into(), accepted: true }],
        }));
        assert_eq!(find_allowed_command_while_paused(&queue), Some(0));
    }

    #[test]
    fn test_find_allowed_command_finds_abort() {
        let mut queue = VecDeque::new();
        queue.push_back(make_request(ChatCommand::UserMessage {
            content: json!("hi"),
            attachments: vec![],
        }));
        queue.push_back(make_request(ChatCommand::UserMessage {
            content: json!("another"),
            attachments: vec![],
        }));
        queue.push_back(make_request(ChatCommand::Abort {}));
        assert_eq!(find_allowed_command_while_paused(&queue), Some(2));
    }

    #[test]
    fn test_find_allowed_command_returns_first_match() {
        let mut queue = VecDeque::new();
        queue.push_back(make_request(ChatCommand::Abort {}));
        queue.push_back(make_request(ChatCommand::ToolDecision {
            tool_call_id: "tc1".into(),
            accepted: true,
        }));
        assert_eq!(find_allowed_command_while_paused(&queue), Some(0));
    }

    #[test]
    fn test_apply_setparams_model() {
        let mut thread = ThreadParams::default();
        thread.model = "old-model".into();
        let patch = json!({"model": "new-model"});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert_eq!(thread.model, "new-model");
    }

    #[test]
    fn test_apply_setparams_no_change_same_value() {
        let mut thread = ThreadParams::default();
        thread.model = "gpt-4".into();
        let patch = json!({"model": "gpt-4"});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(!changed);
    }

    #[test]
    fn test_apply_setparams_mode() {
        let mut thread = ThreadParams::default();
        let patch = json!({"mode": "NO_TOOLS"});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert_eq!(thread.mode, "NO_TOOLS");
    }

    #[test]
    fn test_apply_setparams_boost_reasoning() {
        let mut thread = ThreadParams::default();
        let patch = json!({"boost_reasoning": true});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert!(thread.boost_reasoning);
    }

    #[test]
    fn test_apply_setparams_tool_use() {
        let mut thread = ThreadParams::default();
        let patch = json!({"tool_use": "disabled"});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert_eq!(thread.tool_use, "disabled");
    }

    #[test]
    fn test_apply_setparams_context_tokens_cap() {
        let mut thread = ThreadParams::default();
        let patch = json!({"context_tokens_cap": 4096});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert_eq!(thread.context_tokens_cap, Some(4096));
    }

    #[test]
    fn test_apply_setparams_context_tokens_cap_null() {
        let mut thread = ThreadParams::default();
        thread.context_tokens_cap = Some(4096);
        let patch = json!({"context_tokens_cap": null});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert!(thread.context_tokens_cap.is_none());
    }

    #[test]
    fn test_apply_setparams_include_project_info() {
        let mut thread = ThreadParams::default();
        let patch = json!({"include_project_info": false});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert!(!thread.include_project_info);
    }

    #[test]
    fn test_apply_setparams_checkpoints_enabled() {
        let mut thread = ThreadParams::default();
        let patch = json!({"checkpoints_enabled": false});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert!(!thread.checkpoints_enabled);
    }

    #[test]
    fn test_apply_setparams_multiple_fields() {
        let mut thread = ThreadParams::default();
        let patch = json!({
            "model": "claude-3",
            "mode": "EXPLORE",
            "boost_reasoning": true,
        });
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(changed);
        assert_eq!(thread.model, "claude-3");
        assert_eq!(thread.mode, "EXPLORE");
        assert!(thread.boost_reasoning);
    }

    #[test]
    fn test_apply_setparams_sanitizes_patch() {
        let mut thread = ThreadParams::default();
        let patch = json!({
            "model": "gpt-4",
            "type": "set_params",
            "chat_id": "chat-123",
            "seq": "42"
        });
        let (_, sanitized) = apply_setparams_patch(&mut thread, &patch);
        assert!(sanitized.get("type").is_none());
        assert!(sanitized.get("chat_id").is_none());
        assert!(sanitized.get("seq").is_none());
        assert!(sanitized.get("model").is_some());
    }

    #[test]
    fn test_apply_setparams_empty_patch() {
        let mut thread = ThreadParams::default();
        let original_model = thread.model.clone();
        let patch = json!({});
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(!changed);
        assert_eq!(thread.model, original_model);
    }

    #[test]
    fn test_apply_setparams_invalid_types_ignored() {
        let mut thread = ThreadParams::default();
        thread.model = "original".into();
        let patch = json!({
            "model": 123,
            "boost_reasoning": "not_a_bool",
        });
        let (changed, _) = apply_setparams_patch(&mut thread, &patch);
        assert!(!changed);
        assert_eq!(thread.model, "original");
    }
}
