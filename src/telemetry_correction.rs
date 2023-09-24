use tracing::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use std::sync::RwLock as StdRwLock;
use std::collections::HashMap;
use reqwest_eventsource::Event;
use futures::StreamExt;
use async_stream::stream;
use serde_json::json;
use crate::caps::CodeAssistantCaps;
use crate::call_validation;
use serde::Deserialize;
use serde::Serialize;
use crate::global_context;


// How it works:
// 1. Rust returns {"snippet_telemetry_id":101,"choices":[{"code_completion":"\n    return \"Hello World!\"\n"}] ...}
// ?. IDE detects accept, sends /v1/completion-accepted with {"snippet_telemetry_id":101}
// 3. LSP looks at file changes (LSP can be replaced with reaction to a next completion?)
// 4. Changes are translated to "after_walkaway_remaining50to95" etc



#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SnippetTelemetry {
    pub inputs: call_validation::CodeCompletionInputs,
    pub grey_text: String,
    pub corrected_by_user: String,
    pub remaining_percent: f64,
}


// pub fn sources_changed(
//     Arc<StdRwLock<Storage>>,
// {
//     Storage
// }
