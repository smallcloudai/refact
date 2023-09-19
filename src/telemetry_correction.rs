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


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TelemetryNetwork {
    pub url: String,           // communication with url
    pub scope: String,         // in relation to what
    pub success: bool,
    pub error_message: String, // empty if no error
}

pub fn key_telemetry_network(rec: &TelemetryNetwork) -> String {
    format!("{} {} {} {}", rec.url, rec.scope, rec.success, rec.error_message)
}


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TelemetryCompletion {
    pub language: String,
    pub multiline: bool,
    pub generated_chars: usize,
    pub status: String,    // "accepted", "rejected", "corrected"
    pub remaining_percent: f64,
}


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TelemetryCorrection {
    pub inputs: call_validation::CodeCompletionInputs,
    pub grey_text: String,
    pub corrected_by_user: String,
    pub remaining_percent: f64,
}


pub struct Storage {
    pub last_flushed_ts: u64,
    pub tele_net: Vec<TelemetryNetwork>,
    pub tele_comp: Vec<TelemetryCompletion>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            last_flushed_ts: 0,
            tele_net: Vec::new(),
            tele_comp: Vec::new(),
        }
    }
}


pub fn flush_telemetry(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    unimplemented!();
    // telemetry_basic_dest
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

