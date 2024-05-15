use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use serde::{Serialize, Deserialize};

use tokio::sync::RwLock as ARwLock;
use tracing::debug;

use crate::global_context;
use crate::completion_cache;
use crate::call_validation::CodeCompletionPost;
use crate::telemetry::telemetry_structs;
use crate::telemetry::basic_robot_human;
use crate::telemetry::basic_comp_counters;
use crate::telemetry::telemetry_structs::SnippetTracker;
use crate::telemetry::utils;


// How it works:
// 1. Rust returns {"snippet_telemetry_id":101,"choices":[{"code_completion":"\n    return \"Hello World!\"\n"}] ...}
// 2. IDE detects accept, sends /v1/completion-accepted with {"snippet_telemetry_id":101}
// 3. LSP looks at file changes
// 4. Changes are translated to base telemetry counters


#[derive(Debug, Clone)]
pub struct SaveSnippet {
    // Purpose is to aggregate this struct to a scratchpad
    pub storage_arc: Arc<StdRwLock<telemetry_structs::Storage>>,
    pub post: CodeCompletionPost,
}

impl SaveSnippet {
    pub fn new(
        storage_arc: Arc<StdRwLock<telemetry_structs::Storage>>,
        post: &CodeCompletionPost
    ) -> Self {
        SaveSnippet {
            storage_arc,
            post: post.clone(),
        }
    }
}

fn snippet_register(
    ss: &SaveSnippet,
    grey_text: String,
    context_used: bool,
) -> u64 {
    let mut storage_locked = ss.storage_arc.write().unwrap();
    let snippet_telemetry_id = storage_locked.tele_snippet_next_id;
    let mut model = ss.post.model.clone();
    if context_used {
        model = format!("{}+ast", model);
    }
    let snip = SnippetTracker {
        snippet_telemetry_id,
        model,
        inputs: ss.post.inputs.clone(),
        grey_text: grey_text.clone(),
        corrected_by_user: "".to_string(),
        remaining_percentage: -1.,
        created_ts: chrono::Local::now().timestamp(),
        accepted_ts: 0,
        finished_ts: 0,
    };
    storage_locked.tele_snippet_next_id += 1;
    storage_locked.tele_snippets.push(snip);
    snippet_telemetry_id
}

pub fn snippet_register_from_data4cache(
    ss: &SaveSnippet,
    data4cache: &mut completion_cache::CompletionSaveToCache,
    context_used: bool,
) {
    // Convenience function: snippet_telemetry_id should be returned inside a cached answer as well, so there's
    // typically a combination of the two
    if data4cache.completion0_finish_reason.is_empty() {
        return;
    }
    data4cache.completion0_snippet_telemetry_id = Some(snippet_register(&ss, data4cache.completion0_text.clone(), context_used));
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
        snip.accepted_ts = chrono::Local::now().timestamp();
        debug!("snippet_accepted: ID{}: snippet is accepted", snippet_telemetry_id);
        return true;
    }
    return false;
}


pub async fn sources_changed(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    uri: &String,
    text: &String,
) {
    let tele_storage_arc = gcx.read().await.telemetry.clone();
    let mut storage_locked = tele_storage_arc.write().unwrap();

    storage_locked.last_seen_file_texts.insert(uri.clone(), text.clone());
    basic_robot_human::create_robot_human_record_if_not_exists(&mut storage_locked.tele_robot_human, uri, text);

    let mut accepted_snippets = vec![];
    for snip in storage_locked.tele_snippets.iter_mut() {
        if snip.accepted_ts == 0 || !uri.ends_with(&snip.inputs.cursor.file) {
            continue;
        }
        if snip.finished_ts > 0 {
            continue;
        }
        let orig_text = snip.inputs.sources.get(&snip.inputs.cursor.file);
        if !orig_text.is_some() {
            continue;
        }

        // if snip.id is not in the list of finished snippets, add it
        if !accepted_snippets.iter().any(|s: &SnippetTracker| s.snippet_telemetry_id == snip.snippet_telemetry_id) {
            accepted_snippets.push(snip.clone());
            debug!("sources_changed: ID{}: snippet is added to accepted", snip.snippet_telemetry_id);
        }

        let (grey_valid, mut grey_corrected) = utils::if_head_tail_equal_return_added_text(
            orig_text.unwrap(),
            text,
            &snip.grey_text,
        );
        if grey_valid {
            let unchanged_percentage = utils::unchanged_percentage(&grey_corrected, &snip.grey_text);
            grey_corrected = grey_corrected.replace("\r", "");
            snip.corrected_by_user = grey_corrected.clone();
            snip.remaining_percentage = unchanged_percentage;
        } else {
            if snip.remaining_percentage >= 0. {
                snip.finished_ts = chrono::Local::now().timestamp();
                debug!("ID{}: snippet is finished, remaining_percentage={}", snip.snippet_telemetry_id, snip.remaining_percentage);
            } else {
                snip.accepted_ts = 0;  // that will clean up and not send
            }
        }
    }

    for snip in accepted_snippets {
        basic_robot_human::increase_counters_from_accepted_snippet(&mut storage_locked, uri, text, &snip);
        basic_comp_counters::create_data_accumulator_for_accepted_snippet(&mut storage_locked.snippet_data_accumulators, uri, &snip);
    }
    basic_robot_human::on_file_text_changed(&mut storage_locked.tele_robot_human, uri, text);
    basic_comp_counters::on_file_text_changed(&mut storage_locked.snippet_data_accumulators, uri, text);
}
