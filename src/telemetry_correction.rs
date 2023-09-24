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
pub struct Correction {
    pub inputs: call_validation::CodeCompletionInputs,
    pub grey_text: String,
    pub corrected_by_user: String,
    pub remaining_percent: f64,
}


// public scope: string = "";
// public positive: boolean = false;
// public url: string = "";
// public model: string = "";
// public function: string = "";
// public intent: string = "";
// public sources: { [key: string]: string } = {};
// public cursor_file: string = "";
// public cursor_pos0: number = 0;
// public cursor_pos1: number = 0;
// public de_facto_model: string = "";
// public results: { [key: string]: string } = {}; // filled for diff
// public grey_text_explicitly: string = "";       // filled for completion
// public grey_text_edited: string = "";           // user changed something within completion
// public messages: [string, string][] = [];       // filled for chat
// public ts_req: number = 0;
// public ts_presented: number = 0;
// public ts_reacted: number = 0;
// public serial_number: number = 0;
// public accepted: boolean = false;
// public rejected_reason: string = "";
// public unchanged_percentage: number = 0;

