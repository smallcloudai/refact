use tracing::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use std::sync::RwLock as StdRwLock;

use crate::call_validation;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use crate::global_context;
use crate::completion_cache;
use crate::telemetry_storage;
use crate::call_validation::CodeCompletionPost;
use similar::{ChangeTag, TextDiff};


// How it works:
// 1. Rust returns {"snippet_telemetry_id":101,"choices":[{"code_completion":"\n    return \"Hello World!\"\n"}] ...}
// ?. IDE detects accept, sends /v1/completion-accepted with {"snippet_telemetry_id":101}
// 3. LSP looks at file changes (LSP can be replaced with reaction to a next completion?)
// 4. Changes are translated to "after_walkaway_remaining50to95" etc

const SNIP_FINISHED_AFTER : i64 = 300;
const SNIP_TIMEOUT_AFTER : i64 = 30;


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
            storage_arc,
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
    // pub walkaway_ms: u64,
    pub remaining_percentage: f64,
    pub created_ts: i64,
    pub accepted_ts: i64,
    pub finished_ts: i64,
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
        remaining_percentage: 0.0,
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
        snip.accepted_ts = chrono::Local::now().timestamp();
        return true;
    }
    return false;
}

pub async fn sources_changed(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    uri: &String,
    text: &String,
) {
    let tele_storage = gcx.read().await.telemetry.clone();
    let mut storage_locked = tele_storage.write().unwrap();
    for snip in &mut storage_locked.tele_snippets {
        if !snip.accepted || snip.finished_ts > 0 || !uri.ends_with(&snip.inputs.cursor.file) {
            continue;
        }
        let orig_text = snip.inputs.sources.get(&snip.inputs.cursor.file);
        if !orig_text.is_some() {
            continue;
        }
        let (grey_valid, mut grey_corrected) = if_head_tail_equal_return_added_text(
            orig_text.unwrap(),
            text,
            &snip.grey_text,
        );
        info!("grey_valid: {:?}, grey_corrected: {:?}", grey_valid, grey_corrected);
        info!("orig grey_text: {:?}", snip.grey_text);
        if grey_valid {
            let unchanged_percentage = unchanged_percentage(&grey_corrected, &snip.grey_text);
            info!("unchanged_percentage {:.2}", unchanged_percentage);
            grey_corrected = grey_corrected.replace("\r", "");
            snip.corrected_by_user = grey_corrected.clone();
            snip.remaining_percentage = unchanged_percentage;
        } else {
            if !snip.corrected_by_user.is_empty() {
                snip.finished_ts = chrono::Local::now().timestamp();
                info!("snip {} is finished with score={}!", snip.grey_text, snip.remaining_percentage);
            } else {
                info!("snip {} is finished with accepted = false", snip.grey_text);
                snip.accepted = false;
            }
        }
    }
}

pub fn if_head_tail_equal_return_added_text(
    text_a: &String,
    text_b: &String,
    orig_grey_text: &String,
) -> (bool, String) {
    let diff = TextDiff::from_lines(text_a, text_b);
    let mut allow_add_spaces_once = true;
    let is_multiline = orig_grey_text.contains("\n");
    let mut adding_one_block = false;
    let mut added_one_block = false;
    let mut added_text = "".to_string();
    let mut kill_slash_n = false;
    let regex_space_only = regex::Regex::new(r"^\s*$").unwrap();
    let mut deletion_once = "".to_string();
    for c in diff.iter_all_changes() {
        match c.tag() {
            ChangeTag::Delete => {
                // info!("- {}", c.value());
                if adding_one_block {
                    added_one_block = true;
                }
                let whitespace_only = regex_space_only.is_match(&c.value());
                if !whitespace_only {
                    if deletion_once.is_empty() {
                        deletion_once = c.value().clone().to_string();
                        if deletion_once.ends_with("\n") {
                            deletion_once = deletion_once[..deletion_once.len() - 1].to_string();
                        }
                    } else {
                        error!("!whitespace_only");
                        return (false, "".to_string());
                    }
                }
                if c.value().ends_with("\n") {
                    kill_slash_n = true;
                }
            }
            ChangeTag::Insert => {
                // info!("+ {}", c.value());
                let val = c.value().clone();
                let whitespace_only = regex_space_only.is_match(&c.value());

                if !allow_add_spaces_once {
                    error!("!allow_add_spaces_once");
                    return (false, "".to_string());
                }
                if whitespace_only {
                    allow_add_spaces_once = false;
                }
                if added_one_block {
                    error!("added is more then one block!");
                    return (false, "".to_string());
                }
                if !deletion_once.is_empty() && !val.starts_with(&deletion_once.clone()) {
                    info!("!deletion_once.is_empty() && !val.starts_with(&deletion_once.clone())");
                    return (false, "".to_string());
                }

                if adding_one_block && !is_multiline {
                    if !whitespace_only {
                        error!("adding_one_block && !is_multiline && !whitespace_only");
                        return (false, "".to_string());
                    }
                }

                if deletion_once.is_empty() {
                    added_text += val;
                } else {
                    added_text += &val[deletion_once.len()..];
                }
                adding_one_block = true;
            }
            ChangeTag::Equal => {
                // info!("= {}", c.value());
                if adding_one_block {
                    added_one_block = true;
                }
            }
        }
    }
    if kill_slash_n {
        if !added_text.ends_with("\n") {
            // should not normally happen, but who knows
            error!("if_head_tail_equal_return_added_text: added_text does not end with \\n");
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
    let diff = TextDiff::from_chars(text_a, text_b);
    let mut common = 0;
    for c in diff.iter_all_changes() {
        match c.tag() {
            ChangeTag::Delete => {
            }
            ChangeTag::Insert => {
            }
            ChangeTag::Equal => {
                common += c.value().len();
            }
        }
    }
    let largest_of_two = text_a.len().max(text_b.len());
    (common as f64) / (largest_of_two as f64)
}

async fn send_finished_snippets(gcx: Arc<ARwLock<global_context::GlobalContext>>) {
    let tele_storage;
    let now = chrono::Local::now().timestamp();
    let enduser_client_version;
    let api_key: String;
    let caps;
    let mothership_enabled: bool;
    let mut telemetry_corrected_snippets_dest = String::new();
    {
        let cx = gcx.read().await;
        enduser_client_version = cx.cmdline.enduser_client_version.clone();
        tele_storage = cx.telemetry.clone();
        api_key = cx.cmdline.api_key.clone();
        caps = cx.caps.clone();
        mothership_enabled = cx.cmdline.snippet_telemetry;
    }
    if let Some(caps) = &caps {
        telemetry_corrected_snippets_dest = caps.read().unwrap().telemetry_corrected_snippets_dest.clone();
    }

    let mut snips_send: Vec<SnippetTelemetry> = vec![];
    {
        let mut to_remove: Vec<usize> = vec![];
        let mut storage_locked = tele_storage.write().unwrap();
        for (idx, snip) in &mut storage_locked.tele_snippets.iter().enumerate() {
            if snip.accepted && snip.accepted_ts > 0 {
                if snip.finished_ts > 0 {
                    to_remove.push(idx);
                    snips_send.push(snip.clone());
                }
                continue;
            }
            if !snip.accepted && now - snip.created_ts >= SNIP_TIMEOUT_AFTER {
                to_remove.push(idx);
                continue;
            }
        }
        for idx in to_remove.iter().rev() {
            storage_locked.tele_snippets.remove(*idx);
        }
    }

    if !mothership_enabled {
        info!("telemetry snippets sending not enabled, skip");
        return;
    }
    if telemetry_corrected_snippets_dest.is_empty() {
        info!("telemetry_corrected_snippets_dest is empty, skip");
        return;
    }
    if snips_send.is_empty() {
        info!("no snippets to send, skip");
        return;
    }
    info!("sending {} snippets", snips_send.len());

    for snip in snips_send {
        let json_dict = serde_json::to_value(snip).unwrap();
        info!("sending snippet: {:?}", json_dict);
        let big_json_snip = json!({
            "records": [json_dict],
            "ts_start": now,
            "ts_end": chrono::Local::now().timestamp(),
            "teletype": "snippets",
            "enduser_client_version": enduser_client_version,
        });
        let resp_maybe = telemetry_storage::send_telemetry_data(
            big_json_snip.to_string(),
            &telemetry_corrected_snippets_dest,
            &api_key
        ).await;
        if resp_maybe.is_err() {
            error!("snippet send failed: {}", resp_maybe.err().unwrap());
            error!("too bad snippet is lost now");
            continue;
        }
    }
}

pub async fn tele_snip_background_task(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("tele_snip_background_task");
        send_finished_snippets(global_context.clone()).await;
    }
}
