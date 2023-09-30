use tracing::info;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use std::sync::RwLock as StdRwLock;
// use std::collections::HashMap;
// use reqwest_eventsource::Event;
// use futures::StreamExt;
// use async_stream::stream;
// use serde_json::json;
// use crate::caps::CodeAssistantCaps;
use crate::call_validation;
use serde::Deserialize;
use serde::Serialize;
use crate::global_context;
use crate::completion_cache;
use crate::telemetry_storage;
use crate::call_validation::CodeCompletionPost;
use difference;


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
    // Convenience function: snippet_telemetry_id should be returned inside a cached answer as well, so there's
    // typically a combination of the two
    if data4cache.completion0_finish_reason.is_empty() {
        return;
    }
    data4cache.completion0_snippet_telemetry_id = Some(snippet_register(&ss, data4cache.completion0_text.clone()));
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnippetAccepted {
    pub snippet_telemetry_id: u64,
}

pub async fn snippet_accepted(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    snippet_telemetry_id: u64,
) -> bool {
    let tele_storage_arc = gcx.read().await.telemetry.clone();
    let mut storage_locked = tele_storage_arc.write().unwrap();
    let snip = storage_locked.tele_snippets.iter_mut().find(|s| s.snippet_telemetry_id == snippet_telemetry_id);
    if let Some(snip) = snip {
        snip.accepted = true;
        return true;
    }
    return false;
}

pub async fn sources_changed(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    uri: &String,
    text: &String,
) {
    info!("sources_changed: uri: {:?}, text: {:?}", uri, text);
    let tele_storage = gcx.read().await.telemetry.clone();
    let mut storage_locked = tele_storage.write().unwrap();
    //     //  orig1    orig1    orig1
    //     //  orig2    orig2    orig2
    //     //  |        comp1    comp1
    //     //  orig3    comp2    edit
    //     //  orig4    comp3    comp3
    //     //  orig5    orig3    orig3
    //     //           orig4    orig4
    //     // -------------------------------
    //     // Goal: diff orig vs compl, orig vs uedit. If head and tail are the same, then user edit is valid and useful.
    //     // Memorize the last valid user edit. At the point it becomes invalid, save feedback and forget.
    for snip in &mut storage_locked.tele_snippets {
        info!("does {:?} match {:?}", uri, snip.inputs.cursor.file);
        if !uri.ends_with(&snip.inputs.cursor.file) {
            continue;
        }
        let orig_text = snip.inputs.sources.get(&snip.inputs.cursor.file);
        if !orig_text.is_some() {
            continue;
        }
        let (valid1, mut gray_suggested) = if_head_tail_equal_return_added_text(
            orig_text.unwrap(),
            text
        );
        gray_suggested = gray_suggested.replace("\r", "");
        info!("valid1: {:?}, gray_suggested: {:?}", valid1, gray_suggested);
        info!("orig grey_text: {:?}", snip.grey_text);
        let unchanged_percentage = unchanged_percentage(&gray_suggested, &snip.grey_text);
        info!("unchanged_percentage {:.2}", unchanged_percentage);
    }
}

pub fn if_head_tail_equal_return_added_text(
    text_a: &String,
    text_b: &String,
) -> (bool, String) {
    let difference::Changeset { diffs, .. } = difference::Changeset::new(&text_a, &text_b, "\n");
    let mut allow_remove_spaces_once = true;
    let mut added_one_block = false;
    let mut added_text = "".to_string();
    let mut kill_slash_n = false;
    let mut failed = false;
    let regex_space_only = regex::Regex::new(r"^\s*$").unwrap();
    for c in &diffs {
        match *c {
            difference::Difference::Rem(ref z) => {
                if !allow_remove_spaces_once {
                    failed = true;
                }
                allow_remove_spaces_once = false;
                let whitespace_only = regex_space_only.is_match(&z);
                if !whitespace_only {
                    failed = true;
                }
                if z.ends_with("\n") {
                    kill_slash_n = true;
                }
            }
            difference::Difference::Add(ref z) => {
                if added_one_block {
                    failed = true;
                }
                added_one_block = true;
                added_text = z.clone();
            }
            difference::Difference::Same(ref _z) => {
            }
        }
    }
    if failed {
        return (false, "".to_string());
    }
    if kill_slash_n {
        if !added_text.ends_with("\n") {
            // should not normally happen, but who knows
            info!("if_head_tail_equal_return_added_text: added_text does not end with \\n");
            return (false, "".to_string());
        }
        added_text = added_text[..added_text.len() - 1].to_string();
    }
    (true, added_text)
}

pub fn unchanged_percentage(
    text_a: &String,
    text_b: &String,
) -> f64 {
    let char_level = "";
    let difference::Changeset { diffs, .. } = difference::Changeset::new(&text_a, &text_b, char_level);
    let mut common = 0;
    for c in &diffs {
        match *c {
            difference::Difference::Rem(ref _z) => {
            }
            difference::Difference::Add(ref _z) => {
            }
            difference::Difference::Same(ref z) => {
                common += z.len();
            }
        }
    }
    let largest_of_two = text_a.len().max(text_b.len());
    (common as f64) / (largest_of_two as f64)
}
