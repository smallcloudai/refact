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
use crate::completion_cache;
use crate::telemetry_storage;
use crate::call_validation::CodeCompletionPost;


// How it works:
// 1. Rust returns {"snippet_telemetry_id":101,"choices":[{"code_completion":"\n    return \"Hello World!\"\n"}] ...}
// ?. IDE detects accept, sends /v1/completion-accepted with {"snippet_telemetry_id":101}
// 3. LSP looks at file changes (LSP can be replaced with reaction to a next completion?)
// 4. Changes are translated to "after_walkaway_remaining50to95" etc


#[derive(Debug, Clone)]
pub struct SaveSnippet {
    pub storage_arc: Arc<StdRwLock<telemetry_storage::Storage>>,
    pub post: CodeCompletionPost,
}

impl SaveSnippet {
    pub fn new(
        storage_arc: Arc<StdRwLock<telemetry_storage::Storage>>,
        post: &CodeCompletionPost
    ) -> Self {
        SaveSnippet {
            storage_arc: storage_arc,
            post: post.clone(),
        }
    }
}



#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SnippetTelemetry {
    pub snippet_telemetry_id: u64,
    pub inputs: call_validation::CodeCompletionInputs,
    pub grey_text: String,
    pub accepted: bool,
    pub corrected_by_user: String,
    pub remaining_percent: f64,
}

pub fn snippet_register(
    ss: &SaveSnippet,
    grey_text: String,
) -> u64 {
    let mut storage_locked = ss.storage_arc.write().unwrap();
    let snippet_telemetry_id = storage_locked.tele_snippet_next_id;
    let snip = SnippetTelemetry {
        snippet_telemetry_id,
        inputs: ss.post.inputs.clone(),
        grey_text: grey_text.clone(),
        accepted: false,
        corrected_by_user: "".to_string(),
        remaining_percent: 0.0,
    };
    storage_locked.tele_snippet_next_id += 1;
    storage_locked.tele_snippets.push(snip);
    snippet_telemetry_id
}

pub fn snippet_register_from_data4cache(
    ss: &SaveSnippet,
    data4cache: &mut completion_cache::CompletionSaveToCache,
) {
    // Convenience function: snippet_telemetry_id should be returned inside cached answer as well, so there's
    // typically a combination of the two
    if data4cache.completion0_finish_reason.is_empty() {
        return;
    }
    data4cache.completion0_snippet_telemetry_id = Some(snippet_register(&ss, data4cache.completion0_text.clone()));
}

pub fn snippet_accepted(
    storage_arc: Arc<StdRwLock<telemetry_storage::Storage>>,
    snippet_telemetry_id: u64,
) {
    let storage_locked = storage_arc.write().unwrap();

    unimplemented!();
}

pub fn sources_changed(
    storage_arc: Arc<StdRwLock<telemetry_storage::Storage>>,
) {
    unimplemented!();
}
