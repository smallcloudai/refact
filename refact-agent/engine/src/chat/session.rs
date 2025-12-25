use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use serde_json::json;
use tokio::sync::{broadcast, Mutex as AMutex, Notify, RwLock as ARwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::GlobalContext;

use super::types::*;

pub type SessionsMap = Arc<ARwLock<HashMap<String, Arc<AMutex<ChatSession>>>>>;

pub fn create_sessions_map() -> SessionsMap {
    Arc::new(ARwLock::new(HashMap::new()))
}

impl ChatSession {
    pub fn new(chat_id: String) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            chat_id: chat_id.clone(),
            thread: ThreadParams { id: chat_id, ..Default::default() },
            messages: Vec::new(),
            runtime: RuntimeState::default(),
            draft_message: None,
            draft_usage: None,
            command_queue: VecDeque::new(),
            event_seq: 0,
            event_tx,
            recent_request_ids: VecDeque::with_capacity(100),
            abort_flag: Arc::new(AtomicBool::new(false)),
            queue_processor_running: Arc::new(AtomicBool::new(false)),
            queue_notify: Arc::new(Notify::new()),
            last_activity: Instant::now(),
            trajectory_dirty: false,
            trajectory_version: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            closed: false,
            external_reload_pending: false,
        }
    }

    pub fn new_with_trajectory(chat_id: String, messages: Vec<ChatMessage>, thread: ThreadParams, created_at: String) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            chat_id,
            thread,
            messages,
            runtime: RuntimeState::default(),
            draft_message: None,
            draft_usage: None,
            command_queue: VecDeque::new(),
            event_seq: 0,
            event_tx,
            recent_request_ids: VecDeque::with_capacity(100),
            abort_flag: Arc::new(AtomicBool::new(false)),
            queue_processor_running: Arc::new(AtomicBool::new(false)),
            queue_notify: Arc::new(Notify::new()),
            last_activity: Instant::now(),
            external_reload_pending: false,
            trajectory_dirty: false,
            trajectory_version: 0,
            created_at,
            closed: false,
        }
    }

    pub fn increment_version(&mut self) {
        self.trajectory_version += 1;
        self.trajectory_dirty = true;
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_idle_for_cleanup(&self) -> bool {
        self.runtime.state == SessionState::Idle
            && self.command_queue.is_empty()
            && self.last_activity.elapsed() > SESSION_IDLE_TIMEOUT
    }

    pub fn emit(&mut self, event: ChatEvent) {
        self.event_seq += 1;
        let envelope = EventEnvelope {
            chat_id: self.chat_id.clone(),
            seq: self.event_seq,
            event,
        };
        let _ = self.event_tx.send(envelope);
    }

    pub fn snapshot(&self) -> ChatEvent {
        let mut messages = self.messages.clone();
        if self.runtime.state == SessionState::Generating {
            if let Some(ref draft) = self.draft_message {
                messages.push(draft.clone());
            }
        }
        ChatEvent::Snapshot {
            thread: self.thread.clone(),
            runtime: self.runtime.clone(),
            messages,
        }
    }

    pub fn is_duplicate_request(&mut self, request_id: &str) -> bool {
        if self.recent_request_ids.contains(&request_id.to_string()) {
            return true;
        }
        if self.recent_request_ids.len() >= 100 {
            self.recent_request_ids.pop_front();
        }
        self.recent_request_ids.push_back(request_id.to_string());
        false
    }

    pub fn add_message(&mut self, mut message: ChatMessage) {
        if message.message_id.is_empty() {
            message.message_id = Uuid::new_v4().to_string();
        }
        let index = self.messages.len();
        self.messages.push(message.clone());
        self.emit(ChatEvent::MessageAdded { message, index });
        self.increment_version();
        self.touch();
    }

    pub fn update_message(&mut self, message_id: &str, message: ChatMessage) -> Option<usize> {
        if let Some(idx) = self.messages.iter().position(|m| m.message_id == message_id) {
            self.messages[idx] = message.clone();
            self.emit(ChatEvent::MessageUpdated {
                message_id: message_id.to_string(),
                message,
            });
            self.increment_version();
            self.touch();
            return Some(idx);
        }
        None
    }

    pub fn remove_message(&mut self, message_id: &str) -> Option<usize> {
        if let Some(idx) = self.messages.iter().position(|m| m.message_id == message_id) {
            self.messages.remove(idx);
            self.emit(ChatEvent::MessageRemoved { message_id: message_id.to_string() });
            self.increment_version();
            self.touch();
            return Some(idx);
        }
        None
    }

    pub fn truncate_messages(&mut self, from_index: usize) {
        if from_index < self.messages.len() {
            self.messages.truncate(from_index);
            self.emit(ChatEvent::MessagesTruncated { from_index });
            self.increment_version();
            self.touch();
        }
    }

    pub fn set_runtime_state(&mut self, state: SessionState, error: Option<String>) {
        let was_paused = self.runtime.state == SessionState::Paused;
        let had_pause_reasons = !self.runtime.pause_reasons.is_empty();

        self.runtime.state = state;
        self.runtime.paused = state == SessionState::Paused;
        self.runtime.error = error.clone();
        self.runtime.queue_size = self.command_queue.len();

        if state != SessionState::Paused && (was_paused || had_pause_reasons) {
            self.runtime.pause_reasons.clear();
            self.emit(ChatEvent::PauseCleared {});
        }

        self.emit(ChatEvent::RuntimeUpdated {
            state,
            paused: self.runtime.paused,
            error,
            queue_size: self.runtime.queue_size,
        });
    }

    pub fn set_paused_with_reasons(&mut self, reasons: Vec<PauseReason>) {
        self.runtime.pause_reasons = reasons.clone();
        self.emit(ChatEvent::PauseRequired { reasons });
        self.set_runtime_state(SessionState::Paused, None);
    }

    pub fn start_stream(&mut self) -> Option<(String, Arc<AtomicBool>)> {
        if self.runtime.state == SessionState::Generating || self.runtime.state == SessionState::ExecutingTools {
            warn!("Attempted to start stream while already generating/executing");
            return None;
        }
        self.abort_flag.store(false, Ordering::SeqCst);
        let message_id = Uuid::new_v4().to_string();
        self.draft_message = Some(ChatMessage {
            message_id: message_id.clone(),
            role: "assistant".to_string(),
            ..Default::default()
        });
        self.draft_usage = None;
        self.set_runtime_state(SessionState::Generating, None);
        self.emit(ChatEvent::StreamStarted { message_id: message_id.clone() });
        self.touch();
        Some((message_id, self.abort_flag.clone()))
    }

    pub fn emit_stream_delta(&mut self, ops: Vec<DeltaOp>) {
        let message_id = match &mut self.draft_message {
            Some(draft) => {
                for op in &ops {
                    match op {
                        DeltaOp::AppendContent { text } => {
                            match &mut draft.content {
                                ChatContent::SimpleText(s) => s.push_str(text),
                                _ => draft.content = ChatContent::SimpleText(text.clone()),
                            }
                        }
                        DeltaOp::AppendReasoning { text } => {
                            let r = draft.reasoning_content.get_or_insert_with(String::new);
                            r.push_str(text);
                        }
                        DeltaOp::SetToolCalls { tool_calls } => {
                            draft.tool_calls = serde_json::from_value(json!(tool_calls)).ok();
                        }
                        DeltaOp::SetThinkingBlocks { blocks } => {
                            draft.thinking_blocks = Some(blocks.clone());
                        }
                        DeltaOp::AddCitation { citation } => {
                            draft.citations.push(citation.clone());
                        }
                        DeltaOp::SetUsage { usage } => {
                            if let Ok(u) = serde_json::from_value(usage.clone()) {
                                draft.usage = Some(u);
                            }
                        }
                        DeltaOp::MergeExtra { extra } => {
                            draft.extra.extend(extra.clone());
                        }
                    }
                }
                draft.message_id.clone()
            }
            None => return,
        };
        self.emit(ChatEvent::StreamDelta { message_id, ops });
    }

    pub fn finish_stream(&mut self, finish_reason: Option<String>) {
        if let Some(mut draft) = self.draft_message.take() {
            self.emit(ChatEvent::StreamFinished {
                message_id: draft.message_id.clone(),
                finish_reason: finish_reason.clone(),
            });
            draft.finish_reason = finish_reason;
            if let Some(usage) = self.draft_usage.take() {
                draft.usage = Some(usage);
            }
            self.add_message(draft);
        }
        self.set_runtime_state(SessionState::Idle, None);
        self.touch();
    }

    pub fn finish_stream_with_error(&mut self, error: String) {
        if let Some(mut draft) = self.draft_message.take() {
            let has_text_content = match &draft.content {
                ChatContent::SimpleText(s) => !s.is_empty(),
                ChatContent::Multimodal(v) => !v.is_empty(),
                ChatContent::ContextFiles(v) => !v.is_empty(),
            };
            let has_structured_data = draft.tool_calls.as_ref().map_or(false, |tc| !tc.is_empty())
                || draft.reasoning_content.as_ref().map_or(false, |r| !r.is_empty())
                || draft.thinking_blocks.as_ref().map_or(false, |tb| !tb.is_empty())
                || !draft.citations.is_empty()
                || draft.usage.is_some()
                || !draft.extra.is_empty();

            if has_text_content || has_structured_data {
                self.emit(ChatEvent::StreamFinished {
                    message_id: draft.message_id.clone(),
                    finish_reason: Some("error".to_string()),
                });
                draft.finish_reason = Some("error".to_string());
                if let Some(usage) = self.draft_usage.take() {
                    draft.usage = Some(usage);
                }
                self.add_message(draft);
            } else {
                self.emit(ChatEvent::MessageRemoved { message_id: draft.message_id });
            }
        }
        self.set_runtime_state(SessionState::Error, Some(error));
        self.touch();
    }

    pub fn abort_stream(&mut self) {
        self.abort_flag.store(true, Ordering::SeqCst);
        if let Some(draft) = self.draft_message.take() {
            self.emit(ChatEvent::StreamFinished {
                message_id: draft.message_id.clone(),
                finish_reason: Some("abort".to_string()),
            });
            self.emit(ChatEvent::MessageRemoved { message_id: draft.message_id });
        }
        self.draft_usage = None;
        self.set_runtime_state(SessionState::Idle, None);
        self.touch();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.event_tx.subscribe()
    }

    pub fn set_title(&mut self, title: String, is_generated: bool) {
        self.thread.title = title.clone();
        self.thread.is_title_generated = is_generated;
        self.emit(ChatEvent::TitleUpdated { title, is_generated });
        self.increment_version();
        self.touch();
    }

    pub fn validate_tool_decision(&self, tool_call_id: &str) -> bool {
        self.runtime.pause_reasons.iter().any(|r| r.tool_call_id == tool_call_id)
    }

    pub fn process_tool_decisions(&mut self, decisions: &[ToolDecisionItem]) -> Vec<String> {
        let mut accepted_ids = Vec::new();
        let mut denied_ids = Vec::new();

        for decision in decisions {
            if !self.validate_tool_decision(&decision.tool_call_id) {
                warn!("Tool decision for unknown tool_call_id: {}", decision.tool_call_id);
                continue;
            }
            if decision.accepted {
                accepted_ids.push(decision.tool_call_id.clone());
            } else {
                denied_ids.push(decision.tool_call_id.clone());
            }
        }

        self.runtime.pause_reasons.retain(|r| {
            !accepted_ids.contains(&r.tool_call_id) && !denied_ids.contains(&r.tool_call_id)
        });

        if self.runtime.pause_reasons.is_empty() {
            self.set_runtime_state(SessionState::Idle, None);
        }

        accepted_ids
    }
}

pub async fn get_or_create_session_with_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    sessions: &SessionsMap,
    chat_id: &str,
) -> Arc<AMutex<ChatSession>> {
    {
        let sessions_read = sessions.read().await;
        if let Some(session) = sessions_read.get(chat_id) {
            return session.clone();
        }
    }

    let session = if let Some(loaded) = super::trajectories::load_trajectory_for_chat(gcx, chat_id).await {
        info!("Loaded trajectory for chat {} with {} messages", chat_id, loaded.messages.len());
        ChatSession::new_with_trajectory(chat_id.to_string(), loaded.messages, loaded.thread, loaded.created_at)
    } else {
        ChatSession::new(chat_id.to_string())
    };

    let mut sessions_write = sessions.write().await;
    sessions_write
        .entry(chat_id.to_string())
        .or_insert_with(|| Arc::new(AMutex::new(session)))
        .clone()
}

pub fn start_session_cleanup_task(gcx: Arc<ARwLock<GlobalContext>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SESSION_CLEANUP_INTERVAL);
        loop {
            interval.tick().await;

            let sessions = {
                let gcx_locked = gcx.read().await;
                gcx_locked.chat_sessions.clone()
            };

            let candidates: Vec<(String, Arc<AMutex<ChatSession>>)> = {
                let sessions_read = sessions.read().await;
                sessions_read.iter()
                    .map(|(chat_id, session_arc)| (chat_id.clone(), session_arc.clone()))
                    .collect()
            };

            let mut to_cleanup = Vec::new();
            for (chat_id, session_arc) in candidates {
                let session = session_arc.lock().await;
                if session.is_idle_for_cleanup() {
                    drop(session);
                    to_cleanup.push((chat_id, session_arc));
                }
            }

            if to_cleanup.is_empty() {
                continue;
            }

            info!("Cleaning up {} idle sessions", to_cleanup.len());

            for (chat_id, session_arc) in &to_cleanup {
                {
                    let mut session = session_arc.lock().await;
                    session.closed = true;
                    session.queue_notify.notify_one();
                }
                super::trajectories::maybe_save_trajectory(gcx.clone(), session_arc.clone()).await;
                info!("Saved trajectory for closed session {}", chat_id);
            }

            {
                let mut sessions_write = sessions.write().await;
                for (chat_id, _) in &to_cleanup {
                    sessions_write.remove(chat_id);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_session() -> ChatSession {
        ChatSession::new("test-chat".to_string())
    }

    #[test]
    fn test_new_session_initial_state() {
        let session = make_session();
        assert_eq!(session.chat_id, "test-chat");
        assert_eq!(session.thread.id, "test-chat");
        assert_eq!(session.runtime.state, SessionState::Idle);
        assert!(session.messages.is_empty());
        assert!(session.draft_message.is_none());
        assert_eq!(session.event_seq, 0);
        assert!(!session.trajectory_dirty);
    }

    #[test]
    fn test_new_with_trajectory() {
        let msg = ChatMessage {
            role: "user".into(),
            content: ChatContent::SimpleText("hello".into()),
            ..Default::default()
        };
        let thread = ThreadParams {
            id: "traj-1".into(),
            title: "Old Chat".into(),
            ..Default::default()
        };
        let session = ChatSession::new_with_trajectory(
            "traj-1".into(),
            vec![msg.clone()],
            thread,
            "2024-01-01T00:00:00Z".into(),
        );
        assert_eq!(session.chat_id, "traj-1");
        assert_eq!(session.thread.title, "Old Chat");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.created_at, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_emit_increments_seq() {
        let mut session = make_session();
        assert_eq!(session.event_seq, 0);
        session.emit(ChatEvent::PauseCleared {});
        assert_eq!(session.event_seq, 1);
        session.emit(ChatEvent::PauseCleared {});
        assert_eq!(session.event_seq, 2);
    }

    #[test]
    fn test_emit_sends_correct_envelope() {
        let mut session = make_session();
        let mut rx = session.subscribe();
        session.emit(ChatEvent::TitleUpdated {
            title: "Test".into(),
            is_generated: true,
        });
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.chat_id, "test-chat");
        assert_eq!(envelope.seq, 1);
        matches!(envelope.event, ChatEvent::TitleUpdated { .. });
    }

    #[test]
    fn test_snapshot_without_draft() {
        let mut session = make_session();
        session.messages.push(ChatMessage {
            role: "user".into(),
            content: ChatContent::SimpleText("hi".into()),
            ..Default::default()
        });
        let snap = session.snapshot();
        match snap {
            ChatEvent::Snapshot { messages, .. } => {
                assert_eq!(messages.len(), 1);
            }
            _ => panic!("Expected Snapshot"),
        }
    }

    #[test]
    fn test_snapshot_includes_draft_when_generating() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "partial".into() }]);
        let snap = session.snapshot();
        match snap {
            ChatEvent::Snapshot { messages, runtime, .. } => {
                assert_eq!(runtime.state, SessionState::Generating);
                assert_eq!(messages.len(), 1);
                match &messages[0].content {
                    ChatContent::SimpleText(s) => assert_eq!(s, "partial"),
                    _ => panic!("Expected SimpleText"),
                }
            }
            _ => panic!("Expected Snapshot"),
        }
    }

    #[test]
    fn test_is_duplicate_request_detects_duplicates() {
        let mut session = make_session();
        assert!(!session.is_duplicate_request("req-1"));
        assert!(session.is_duplicate_request("req-1"));
        assert!(!session.is_duplicate_request("req-2"));
        assert!(session.is_duplicate_request("req-2"));
    }

    #[test]
    fn test_is_duplicate_request_caps_at_100() {
        let mut session = make_session();
        for i in 0..100 {
            session.is_duplicate_request(&format!("req-{}", i));
        }
        assert_eq!(session.recent_request_ids.len(), 100);
        session.is_duplicate_request("req-100");
        assert_eq!(session.recent_request_ids.len(), 100);
        assert!(!session.recent_request_ids.contains(&"req-0".to_string()));
        assert!(session.recent_request_ids.contains(&"req-100".to_string()));
    }

    #[test]
    fn test_add_message_generates_id_if_empty() {
        let mut session = make_session();
        let msg = ChatMessage {
            role: "user".into(),
            content: ChatContent::SimpleText("hi".into()),
            ..Default::default()
        };
        session.add_message(msg);
        assert!(!session.messages[0].message_id.is_empty());
        assert!(session.trajectory_dirty);
    }

    #[test]
    fn test_add_message_preserves_existing_id() {
        let mut session = make_session();
        let msg = ChatMessage {
            message_id: "custom-id".into(),
            role: "user".into(),
            content: ChatContent::SimpleText("hi".into()),
            ..Default::default()
        };
        session.add_message(msg);
        assert_eq!(session.messages[0].message_id, "custom-id");
    }

    #[test]
    fn test_update_message_returns_index() {
        let mut session = make_session();
        let msg = ChatMessage {
            message_id: "m1".into(),
            role: "user".into(),
            content: ChatContent::SimpleText("original".into()),
            ..Default::default()
        };
        session.messages.push(msg);
        let updated = ChatMessage {
            message_id: "m1".into(),
            role: "user".into(),
            content: ChatContent::SimpleText("updated".into()),
            ..Default::default()
        };
        let idx = session.update_message("m1", updated);
        assert_eq!(idx, Some(0));
        match &session.messages[0].content {
            ChatContent::SimpleText(s) => assert_eq!(s, "updated"),
            _ => panic!("Expected SimpleText"),
        }
    }

    #[test]
    fn test_update_message_unknown_id_returns_none() {
        let mut session = make_session();
        let msg = ChatMessage::default();
        assert!(session.update_message("unknown", msg).is_none());
    }

    #[test]
    fn test_remove_message_returns_index() {
        let mut session = make_session();
        session.messages.push(ChatMessage {
            message_id: "m1".into(),
            ..Default::default()
        });
        session.messages.push(ChatMessage {
            message_id: "m2".into(),
            ..Default::default()
        });
        let idx = session.remove_message("m1");
        assert_eq!(idx, Some(0));
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].message_id, "m2");
    }

    #[test]
    fn test_remove_message_unknown_id_returns_none() {
        let mut session = make_session();
        assert!(session.remove_message("unknown").is_none());
    }

    #[test]
    fn test_truncate_messages() {
        let mut session = make_session();
        for i in 0..5 {
            session.messages.push(ChatMessage {
                message_id: format!("m{}", i),
                ..Default::default()
            });
        }
        session.truncate_messages(2);
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[1].message_id, "m1");
    }

    #[test]
    fn test_truncate_beyond_length_is_noop() {
        let mut session = make_session();
        session.messages.push(ChatMessage::default());
        let version_before = session.trajectory_version;
        session.truncate_messages(10);
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.trajectory_version, version_before);
    }

    #[test]
    fn test_start_stream_returns_message_id() {
        let mut session = make_session();
        let result = session.start_stream();
        assert!(result.is_some());
        let (msg_id, abort_flag) = result.unwrap();
        assert!(!msg_id.is_empty());
        assert!(!abort_flag.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(session.runtime.state, SessionState::Generating);
        assert!(session.draft_message.is_some());
    }

    #[test]
    fn test_start_stream_fails_if_already_generating() {
        let mut session = make_session();
        session.start_stream();
        let result = session.start_stream();
        assert!(result.is_none());
    }

    #[test]
    fn test_start_stream_fails_if_executing_tools() {
        let mut session = make_session();
        session.set_runtime_state(SessionState::ExecutingTools, None);
        let result = session.start_stream();
        assert!(result.is_none());
    }

    #[test]
    fn test_emit_stream_delta_appends_content() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "Hello".into() }]);
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: " World".into() }]);
        let draft = session.draft_message.as_ref().unwrap();
        match &draft.content {
            ChatContent::SimpleText(s) => assert_eq!(s, "Hello World"),
            _ => panic!("Expected SimpleText"),
        }
    }

    #[test]
    fn test_emit_stream_delta_appends_reasoning() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendReasoning { text: "think".into() }]);
        session.emit_stream_delta(vec![DeltaOp::AppendReasoning { text: "ing".into() }]);
        let draft = session.draft_message.as_ref().unwrap();
        assert_eq!(draft.reasoning_content.as_ref().unwrap(), "thinking");
    }

    #[test]
    fn test_emit_stream_delta_sets_tool_calls() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::SetToolCalls {
            tool_calls: vec![json!({"id":"tc1","type":"function","function":{"name":"test","arguments":"{}"}})],
        }]);
        let draft = session.draft_message.as_ref().unwrap();
        assert!(draft.tool_calls.is_some());
        assert_eq!(draft.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_emit_stream_delta_without_draft_is_noop() {
        let mut session = make_session();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "x".into() }]);
        assert!(session.draft_message.is_none());
    }

    #[test]
    fn test_finish_stream_adds_message() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "done".into() }]);
        session.finish_stream(Some("stop".into()));
        assert!(session.draft_message.is_none());
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].finish_reason, Some("stop".into()));
        assert_eq!(session.runtime.state, SessionState::Idle);
    }

    #[test]
    fn test_finish_stream_with_error_keeps_content() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "partial".into() }]);
        session.finish_stream_with_error("timeout".into());
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].finish_reason, Some("error".into()));
        assert_eq!(session.runtime.state, SessionState::Error);
        assert_eq!(session.runtime.error, Some("timeout".into()));
    }

    #[test]
    fn test_finish_stream_with_error_keeps_structured_data() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::SetToolCalls {
            tool_calls: vec![json!({"id":"tc1","type":"function","function":{"name":"test","arguments":"{}"}})],
        }]);
        session.finish_stream_with_error("error".into());
        assert_eq!(session.messages.len(), 1);
    }

    #[test]
    fn test_finish_stream_with_error_removes_empty_draft() {
        let mut session = make_session();
        let mut rx = session.subscribe();
        session.start_stream();
        session.finish_stream_with_error("error".into());
        assert!(session.messages.is_empty());
        let mut found_removed = false;
        while let Ok(env) = rx.try_recv() {
            if matches!(env.event, ChatEvent::MessageRemoved { .. }) {
                found_removed = true;
            }
        }
        assert!(found_removed);
    }

    #[test]
    fn test_abort_stream() {
        let mut session = make_session();
        session.start_stream();
        session.emit_stream_delta(vec![DeltaOp::AppendContent { text: "partial".into() }]);
        session.abort_stream();
        assert!(session.draft_message.is_none());
        assert!(session.messages.is_empty());
        assert!(session.abort_flag.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(session.runtime.state, SessionState::Idle);
    }

    #[test]
    fn test_set_runtime_state_clears_pause_on_transition() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        session.set_runtime_state(SessionState::Paused, None);
        assert!(!session.runtime.pause_reasons.is_empty());
        session.set_runtime_state(SessionState::Idle, None);
        assert!(session.runtime.pause_reasons.is_empty());
    }

    #[test]
    fn test_set_paused_with_reasons() {
        let mut session = make_session();
        let mut rx = session.subscribe();
        let reasons = vec![PauseReason {
            reason_type: "confirmation".into(),
            command: "shell".into(),
            rule: "ask".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        }];
        session.set_paused_with_reasons(reasons.clone());
        assert_eq!(session.runtime.state, SessionState::Paused);
        assert_eq!(session.runtime.pause_reasons.len(), 1);
        let mut found_pause_required = false;
        while let Ok(env) = rx.try_recv() {
            if matches!(env.event, ChatEvent::PauseRequired { .. }) {
                found_pause_required = true;
            }
        }
        assert!(found_pause_required);
    }

    #[test]
    fn test_set_title() {
        let mut session = make_session();
        let mut rx = session.subscribe();
        session.set_title("New Title".into(), true);
        assert_eq!(session.thread.title, "New Title");
        assert!(session.thread.is_title_generated);
        assert!(session.trajectory_dirty);
        let mut found_title = false;
        while let Ok(env) = rx.try_recv() {
            if let ChatEvent::TitleUpdated { title, is_generated } = env.event {
                assert_eq!(title, "New Title");
                assert!(is_generated);
                found_title = true;
            }
        }
        assert!(found_title);
    }

    #[test]
    fn test_validate_tool_decision() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        assert!(session.validate_tool_decision("tc1"));
        assert!(!session.validate_tool_decision("unknown"));
    }

    #[test]
    fn test_process_tool_decisions_accepts() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc2".into(),
            integr_config_path: None,
        });
        session.set_runtime_state(SessionState::Paused, None);
        let accepted = session.process_tool_decisions(&[
            ToolDecisionItem { tool_call_id: "tc1".into(), accepted: true },
        ]);
        assert_eq!(accepted, vec!["tc1"]);
        assert_eq!(session.runtime.pause_reasons.len(), 1);
        assert_eq!(session.runtime.state, SessionState::Paused);
    }

    #[test]
    fn test_process_tool_decisions_denies() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        session.set_runtime_state(SessionState::Paused, None);
        let accepted = session.process_tool_decisions(&[
            ToolDecisionItem { tool_call_id: "tc1".into(), accepted: false },
        ]);
        assert!(accepted.is_empty());
        assert!(session.runtime.pause_reasons.is_empty());
        assert_eq!(session.runtime.state, SessionState::Idle);
    }

    #[test]
    fn test_process_tool_decisions_ignores_unknown() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        session.set_runtime_state(SessionState::Paused, None);
        let accepted = session.process_tool_decisions(&[
            ToolDecisionItem { tool_call_id: "unknown".into(), accepted: true },
        ]);
        assert!(accepted.is_empty());
        assert_eq!(session.runtime.pause_reasons.len(), 1);
    }

    #[test]
    fn test_process_tool_decisions_transitions_to_idle_when_empty() {
        let mut session = make_session();
        session.runtime.pause_reasons.push(PauseReason {
            reason_type: "test".into(),
            command: "cmd".into(),
            rule: "rule".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: None,
        });
        session.set_runtime_state(SessionState::Paused, None);
        session.process_tool_decisions(&[
            ToolDecisionItem { tool_call_id: "tc1".into(), accepted: true },
        ]);
        assert!(session.runtime.pause_reasons.is_empty());
        assert_eq!(session.runtime.state, SessionState::Idle);
    }

    #[test]
    fn test_increment_version() {
        let mut session = make_session();
        assert_eq!(session.trajectory_version, 0);
        assert!(!session.trajectory_dirty);
        session.increment_version();
        assert_eq!(session.trajectory_version, 1);
        assert!(session.trajectory_dirty);
    }

    #[test]
    fn test_create_sessions_map() {
        let map = create_sessions_map();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let read = map.read().await;
            assert!(read.is_empty());
        });
    }
}
