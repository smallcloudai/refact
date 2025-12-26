use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Notify};
use uuid::Uuid;

use crate::call_validation::{ChatMessage, ChatUsage};

pub const MAX_QUEUE_SIZE: usize = 100;
pub const SESSION_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
pub const SESSION_CLEANUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);
pub const STREAM_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
pub const STREAM_TOTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15 * 60);
pub const STREAM_HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Generating,
    ExecutingTools,
    Paused,
    WaitingIde,
    Error,
}

impl Default for SessionState {
    fn default() -> Self { SessionState::Idle }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadParams {
    pub id: String,
    pub title: String,
    pub model: String,
    pub mode: String,
    pub tool_use: String,
    pub boost_reasoning: bool,
    pub context_tokens_cap: Option<usize>,
    pub include_project_info: bool,
    pub checkpoints_enabled: bool,
    #[serde(default = "default_use_compression")]
    pub use_compression: bool,
    #[serde(default)]
    pub is_title_generated: bool,
}

fn default_use_compression() -> bool { true }

impl Default for ThreadParams {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Chat".to_string(),
            model: String::new(),
            mode: "AGENT".to_string(),
            tool_use: "agent".to_string(),
            boost_reasoning: false,
            context_tokens_cap: None,
            include_project_info: true,
            checkpoints_enabled: true,
            use_compression: true,
            is_title_generated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    pub state: SessionState,
    pub paused: bool,
    pub error: Option<String>,
    pub queue_size: usize,
    #[serde(default)]
    pub pause_reasons: Vec<PauseReason>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            state: SessionState::Idle,
            paused: false,
            error: None,
            queue_size: 0,
            pause_reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseReason {
    #[serde(rename = "type")]
    pub reason_type: String,
    pub command: String,
    pub rule: String,
    pub tool_call_id: String,
    pub integr_config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Snapshot {
        thread: ThreadParams,
        runtime: RuntimeState,
        messages: Vec<ChatMessage>,
    },
    ThreadUpdated {
        #[serde(flatten)]
        params: serde_json::Value,
    },
    RuntimeUpdated {
        state: SessionState,
        paused: bool,
        error: Option<String>,
        queue_size: usize,
    },
    TitleUpdated {
        title: String,
        is_generated: bool,
    },
    MessageAdded {
        message: ChatMessage,
        index: usize,
    },
    MessageUpdated {
        message_id: String,
        message: ChatMessage,
    },
    MessageRemoved {
        message_id: String,
    },
    MessagesTruncated {
        from_index: usize,
    },
    StreamStarted {
        message_id: String,
    },
    StreamDelta {
        message_id: String,
        ops: Vec<DeltaOp>,
    },
    StreamFinished {
        message_id: String,
        finish_reason: Option<String>,
    },
    PauseRequired {
        reasons: Vec<PauseReason>,
    },
    PauseCleared {},
    IdeToolRequired {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    SubchatUpdate {
        tool_call_id: String,
        subchat_id: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attached_files: Vec<String>,
    },
    Ack {
        client_request_id: String,
        accepted: bool,
        result: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum DeltaOp {
    AppendContent { text: String },
    AppendReasoning { text: String },
    SetToolCalls { tool_calls: Vec<serde_json::Value> },
    SetThinkingBlocks { blocks: Vec<serde_json::Value> },
    AddCitation { citation: serde_json::Value },
    SetUsage { usage: serde_json::Value },
    MergeExtra { extra: serde_json::Map<String, serde_json::Value> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub chat_id: String,
    #[serde(serialize_with = "serialize_seq_as_string", deserialize_with = "deserialize_seq_from_string")]
    pub seq: u64,
    #[serde(flatten)]
    pub event: ChatEvent,
}

fn serialize_seq_as_string<S>(seq: &u64, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(&seq.to_string())
}

fn deserialize_seq_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where D: serde::Deserializer<'de> {
    use serde::de::Error;
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCommand {
    UserMessage {
        content: serde_json::Value,
        #[serde(default)]
        attachments: Vec<serde_json::Value>,
    },
    RetryFromIndex {
        index: usize,
        content: serde_json::Value,
        #[serde(default)]
        attachments: Vec<serde_json::Value>,
    },
    SetParams {
        patch: serde_json::Value,
    },
    Abort {},
    ToolDecision {
        tool_call_id: String,
        accepted: bool,
    },
    ToolDecisions {
        decisions: Vec<ToolDecisionItem>,
    },
    IdeToolResult {
        tool_call_id: String,
        content: String,
        #[serde(default)]
        tool_failed: bool,
    },
    UpdateMessage {
        message_id: String,
        content: serde_json::Value,
        #[serde(default)]
        attachments: Vec<serde_json::Value>,
        #[serde(default)]
        regenerate: bool,
    },
    RemoveMessage {
        message_id: String,
        #[serde(default)]
        regenerate: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDecisionItem {
    pub tool_call_id: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub client_request_id: String,
    #[serde(flatten)]
    pub command: ChatCommand,
}

pub struct ChatSession {
    pub chat_id: String,
    pub thread: ThreadParams,
    pub messages: Vec<ChatMessage>,
    pub runtime: RuntimeState,
    pub draft_message: Option<ChatMessage>,
    pub draft_usage: Option<ChatUsage>,
    pub command_queue: VecDeque<CommandRequest>,
    pub event_seq: u64,
    pub event_tx: broadcast::Sender<EventEnvelope>,
    pub recent_request_ids: VecDeque<String>,
    pub abort_flag: Arc<AtomicBool>,
    pub queue_processor_running: Arc<AtomicBool>,
    pub queue_notify: Arc<Notify>,
    pub last_activity: Instant,
    pub trajectory_dirty: bool,
    pub trajectory_version: u64,
    pub created_at: String,
    pub closed: bool,
    pub external_reload_pending: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_state_default() {
        assert_eq!(SessionState::default(), SessionState::Idle);
    }

    #[test]
    fn test_session_state_serde() {
        let state = SessionState::Generating;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"generating\"");

        let parsed: SessionState = serde_json::from_str("\"executing_tools\"").unwrap();
        assert_eq!(parsed, SessionState::ExecutingTools);
    }

    #[test]
    fn test_thread_params_default() {
        let params = ThreadParams::default();
        assert_eq!(params.title, "New Chat");
        assert_eq!(params.mode, "AGENT");
        assert_eq!(params.tool_use, "agent");
        assert!(!params.boost_reasoning);
        assert!(params.include_project_info);
        assert!(params.checkpoints_enabled);
        assert!(!params.is_title_generated);
        assert!(params.context_tokens_cap.is_none());
        assert!(!params.id.is_empty());
    }

    #[test]
    fn test_runtime_state_default() {
        let runtime = RuntimeState::default();
        assert_eq!(runtime.state, SessionState::Idle);
        assert!(!runtime.paused);
        assert!(runtime.error.is_none());
        assert_eq!(runtime.queue_size, 0);
        assert!(runtime.pause_reasons.is_empty());
    }

    #[test]
    fn test_event_envelope_seq_serializes_as_string() {
        let envelope = EventEnvelope {
            chat_id: "test-123".to_string(),
            seq: 42,
            event: ChatEvent::PauseCleared {},
        };
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["seq"], "42");
        assert_eq!(json["chat_id"], "test-123");
    }

    #[test]
    fn test_event_envelope_seq_deserializes_from_string() {
        let json = r#"{"chat_id":"abc","seq":"999","type":"pause_cleared"}"#;
        let envelope: EventEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.seq, 999);
        assert_eq!(envelope.chat_id, "abc");
    }

    #[test]
    fn test_event_envelope_invalid_seq_fails() {
        let json = r#"{"chat_id":"abc","seq":"not_a_number","type":"pause_cleared"}"#;
        let result: Result<EventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_command_user_message_defaults() {
        let json = r#"{"type":"user_message","content":"hello"}"#;
        let cmd: ChatCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ChatCommand::UserMessage { content, attachments } => {
                assert_eq!(content, json!("hello"));
                assert!(attachments.is_empty());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_chat_command_ide_tool_result_defaults() {
        let json = r#"{"type":"ide_tool_result","tool_call_id":"tc1","content":"result"}"#;
        let cmd: ChatCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ChatCommand::IdeToolResult { tool_call_id, content, tool_failed } => {
                assert_eq!(tool_call_id, "tc1");
                assert_eq!(content, "result");
                assert!(!tool_failed);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_chat_command_update_message_defaults() {
        let json = r#"{"type":"update_message","message_id":"m1","content":"new"}"#;
        let cmd: ChatCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ChatCommand::UpdateMessage { message_id, content, attachments, regenerate } => {
                assert_eq!(message_id, "m1");
                assert_eq!(content, json!("new"));
                assert!(attachments.is_empty());
                assert!(!regenerate);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_chat_command_remove_message_defaults() {
        let json = r#"{"type":"remove_message","message_id":"m1"}"#;
        let cmd: ChatCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ChatCommand::RemoveMessage { message_id, regenerate } => {
                assert_eq!(message_id, "m1");
                assert!(!regenerate);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_chat_command_all_variants_roundtrip() {
        let commands = vec![
            json!({"type":"user_message","content":"hi","attachments":[]}),
            json!({"type":"retry_from_index","index":2,"content":"retry","attachments":[]}),
            json!({"type":"set_params","patch":{"title":"New"}}),
            json!({"type":"abort"}),
            json!({"type":"tool_decision","tool_call_id":"tc1","accepted":true}),
            json!({"type":"tool_decisions","decisions":[{"tool_call_id":"tc1","accepted":false}]}),
            json!({"type":"ide_tool_result","tool_call_id":"tc1","content":"ok","tool_failed":false}),
            json!({"type":"update_message","message_id":"m1","content":"x","attachments":[],"regenerate":true}),
            json!({"type":"remove_message","message_id":"m1","regenerate":false}),
        ];
        for cmd_json in commands {
            let cmd: ChatCommand = serde_json::from_value(cmd_json.clone()).unwrap();
            let roundtrip = serde_json::to_value(&cmd).unwrap();
            assert_eq!(roundtrip["type"], cmd_json["type"]);
        }
    }

    #[test]
    fn test_delta_op_serde() {
        let ops = vec![
            DeltaOp::AppendContent { text: "hello".into() },
            DeltaOp::AppendReasoning { text: "thinking".into() },
            DeltaOp::SetToolCalls { tool_calls: vec![json!({"id":"1"})] },
            DeltaOp::SetThinkingBlocks { blocks: vec![json!({"type":"thinking"})] },
            DeltaOp::AddCitation { citation: json!({"url":"http://x"}) },
            DeltaOp::SetUsage { usage: json!({"total_tokens":100}) },
            DeltaOp::MergeExtra { extra: serde_json::Map::new() },
        ];
        for op in ops {
            let json = serde_json::to_value(&op).unwrap();
            let parsed: DeltaOp = serde_json::from_value(json).unwrap();
            assert_eq!(
                serde_json::to_string(&op).unwrap(),
                serde_json::to_string(&parsed).unwrap()
            );
        }
    }

    #[test]
    fn test_chat_event_snapshot_serde() {
        let event = ChatEvent::Snapshot {
            thread: ThreadParams::default(),
            runtime: RuntimeState::default(),
            messages: vec![],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "snapshot");
        let parsed: ChatEvent = serde_json::from_value(json).unwrap();
        matches!(parsed, ChatEvent::Snapshot { .. });
    }

    #[test]
    fn test_chat_event_stream_delta_serde() {
        let event = ChatEvent::StreamDelta {
            message_id: "m1".into(),
            ops: vec![DeltaOp::AppendContent { text: "x".into() }],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "stream_delta");
        assert_eq!(json["message_id"], "m1");
    }

    #[test]
    fn test_pause_reason_serde() {
        let reason = PauseReason {
            reason_type: "confirmation".into(),
            command: "shell".into(),
            rule: "ask".into(),
            tool_call_id: "tc1".into(),
            integr_config_path: Some("/path".into()),
        };
        let json = serde_json::to_value(&reason).unwrap();
        assert_eq!(json["type"], "confirmation");
        assert_eq!(json["integr_config_path"], "/path");
    }

    #[test]
    fn test_command_request_flattens_command() {
        let req = CommandRequest {
            client_request_id: "req-1".into(),
            command: ChatCommand::Abort {},
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["client_request_id"], "req-1");
        assert_eq!(json["type"], "abort");
    }
}
