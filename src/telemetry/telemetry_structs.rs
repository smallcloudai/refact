use serde::{Deserialize, Serialize};

use crate::call_validation::CodeCompletionInputs;
use crate::telemetry::utils;


#[derive(Debug)]
pub struct Storage {
    pub last_flushed_ts: i64,
    pub tele_net: Vec<TelemetryNetwork>,
    pub tele_robot_human: Vec<TeleRobotHumanAccum>,
    pub tele_snippets: Vec<SnippetTracker>,
    pub tele_snippet_next_id: u64,
    pub snippet_data_accumulators: Vec<TeleCompletionAccum>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            last_flushed_ts: chrono::Local::now().timestamp(),
            tele_net: Vec::new(),
            tele_robot_human: Vec::new(),
            tele_snippets: Vec::new(),
            tele_snippet_next_id: 100,
            snippet_data_accumulators: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TelemetryNetwork {
    pub url: String,           // communication with url
    pub scope: String,         // in relation to what
    pub success: bool,
    pub error_message: String, // empty if no error
}

impl TelemetryNetwork {
    pub fn new(url: String, scope: String, success: bool, error_message: String) -> Self {
        Self {
            url,
            scope,
            success,
            error_message,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SnippetTracker {
    // Sent directly if snippet telemetry is enabled
    pub snippet_telemetry_id: u64,
    pub model: String,
    pub inputs: CodeCompletionInputs,
    pub grey_text: String,
    pub corrected_by_user: String,
    pub remaining_percentage: f64,
    pub created_ts: i64,
    pub accepted_ts: i64,
    pub finished_ts: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TeleRobotHumanAccum {
    // Internal struct, not sent anywhere
    pub uri: String,
    pub file_extension: String,
    pub model: String,
    // Collected per each file, but compresed by key==(file_extension, model) to remove sensitive information
    pub baseline_text: String,
    pub baseline_updated_ts: i64,
    // Goes from ts to the next ts (see ROBOT_HUMAN_FILE_STATS_UPDATE_EVERY), adds to the counters below
    pub robot_characters_acc_baseline: i64,
    pub robot_characters: i64,
    pub human_characters: i64,
    pub used_snip_ids: Vec<u64>,
}

impl TeleRobotHumanAccum {
    pub fn new(
        uri: String, baseline_text: String
    ) -> Self {
        Self {
            uri: uri.clone(),
            file_extension: utils::extract_extension_or_filename(&uri),
            model: "".to_string(),
            baseline_text,
            baseline_updated_ts: 0,
            robot_characters_acc_baseline: 0,
            robot_characters: 0,
            human_characters: 0,
            used_snip_ids: vec![],
        }
    }
}

#[derive(Debug)]
pub struct TeleCompletionAccum {
    // Internal struct, not sent anywhere. Tracks data for each snippet, converted to basic telemetry (counters) at 30, 60 seconds
    pub snippet_telemetry_id: u64,
    pub uri: String,
    pub file_extension: String,
    pub model: String,
    pub multiline: bool,

    pub init_file_text: String,
    pub init_grey_text: String,
    pub after_30s_remaining: f64,
    pub after_90s_remaining: f64,
    pub after_180s_remaining: f64,
    pub after_360s_remaining: f64,
    pub created_ts: i64,
    pub finished_ts: i64,
}

impl TeleCompletionAccum {
    pub fn new(
        snippet_telemetry_id: u64, uri: String, model: String, init_file_text: String, init_grey_text: String, created_ts: i64
    ) -> Self {
        Self {
            snippet_telemetry_id,
            uri: uri.clone(),
            file_extension: utils::extract_extension_or_filename(&uri),
            multiline: init_grey_text.contains("\n"),

            model,
            init_file_text,
            init_grey_text,
            after_30s_remaining: -1.,
            after_90s_remaining: -1.,
            after_180s_remaining: -1.,
            after_360s_remaining: -1.,
            created_ts,
            finished_ts: 0,
        }
    }
}
