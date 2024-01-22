use serde::{Deserialize, Serialize};
use tracing::{error, info};
use serde_json::json;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::RwLock as StdRwLock;

use tokio::sync::RwLock as ARwLock;

use crate::global_context;
use crate::telemetry::utils;
use crate::telemetry::telemetry_structs;
use crate::telemetry::telemetry_structs::{SnippetTracker, TeleCompletionAccum};


pub fn create_data_accumulator_for_finished_snippet(
    snippet_data_accumulator: &mut Vec<TeleCompletionAccum>,
    uri: &String,
    snip: &SnippetTracker
) {
    if snip.accepted_ts == 0  {
        return;
    }

    // if snip.id is not in the list of finished snippets, add it
    if snippet_data_accumulator.iter().any(|s: &TeleCompletionAccum| s.snippet_telemetry_id == snip.snippet_telemetry_id) {
        return;
    }

    let init_file_text_mb = snip.inputs.sources.get(&snip.inputs.cursor.file);
    if init_file_text_mb.is_none() {
        return;
    }
    let init_file_text = init_file_text_mb.unwrap();

    snippet_data_accumulator.push(TeleCompletionAccum::new(
        snip.snippet_telemetry_id,
        uri.clone(),
        snip.model.clone(),
        init_file_text.clone(),
        snip.grey_text.clone(),
        snip.finished_ts.clone()
    ))
}

pub fn on_file_text_changed(
    snippet_data_accumulator: &mut Vec<TeleCompletionAccum>,
    uri: &String,
    text: &String
) {
    let now = chrono::Local::now().timestamp();
    for comp in snippet_data_accumulator.iter_mut() {
        if !comp.uri.eq(uri) || comp.finished_ts != 0 {
            continue;
        }
        // let remaining_percent = utils::unchanged_percentage_approx(&comp.init_file_text, text, &comp.init_grey_text);
        // error!("Remaining percent={} for snip:\n{}", remaining_percent, comp.init_grey_text);
        // error!("snippet: {}; from_created={}s", comp.init_grey_text, now - comp.created_ts);
        if comp.created_ts + 30 < now && comp.created_ts + 90 > now && comp.after_30s_remaining == -1. {
            comp.after_30s_remaining = utils::unchanged_percentage_approx(&comp.init_file_text, text, &comp.init_grey_text);
            // error!("30s: Remaining percent={} for snip:\n{}", comp.after_30s_remaining, comp.init_grey_text);
        }
        else if comp.created_ts + 90 < now && comp.created_ts + 180 > now && comp.after_90s_remaining == -1. {
            comp.after_90s_remaining = utils::unchanged_percentage_approx(&comp.init_file_text, text, &comp.init_grey_text);
            // error!("90s: Remaining percent={} for snip:\n{}", comp.after_90s_remaining, comp.init_grey_text);
        }
        else if comp.created_ts + 180 < now && comp.created_ts + 360 > now && comp.after_180s_remaining == -1. {
            comp.after_180s_remaining = utils::unchanged_percentage_approx(&comp.init_file_text, text, &comp.init_grey_text);
            // error!("180s: Remaining percent={} for snip:\n{}", comp.after_180s_remaining, comp.init_grey_text);
        }
        else if comp.created_ts + 360 < now && comp.after_360s_remaining == -1. {
            comp.after_360s_remaining = utils::unchanged_percentage_approx(&comp.init_file_text, text, &comp.init_grey_text);
            // error!("360s: Remaining percent={} for snip:\n{}", comp.after_360s_remaining, comp.init_grey_text);
            comp.finished_ts = now;
        }
    }
}


pub async fn compress_tele_completion_to_file(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    let now = chrono::Local::now();
    let cache_dir: PathBuf;
    let storage: Arc<StdRwLock<telemetry_structs::Storage>>;
    let enduser_client_version;
    let mut records = json!([]);
    {
        let cx_locked = cx.read().await;
        storage = cx_locked.telemetry.clone();
        cache_dir = cx_locked.cache_dir.clone();
        enduser_client_version = cx_locked.cmdline.enduser_client_version.clone();

        let mut storage_locked = storage.write().unwrap();
        for rec in compress_into_counters(&storage_locked.snippet_data_accumulators) {
            let json_dict = serde_json::to_value(rec).unwrap();
            records.as_array_mut().unwrap().push(json_dict);
        }
        storage_locked.snippet_data_accumulators.clear();
    }
    if records.as_array().unwrap().is_empty() {
        info!("no completion telemetry to save");
        return;
    }

    let (dir, _) = utils::telemetry_storage_dirs(&cache_dir).await;

    let fn_comp = dir.join(format!("{}-comp.json", now.format("%Y%m%d-%H%M%S")));
    let big_json = json!({
        "records": records,
        "ts_start": now.timestamp(),
        "ts_end": now.timestamp(),
        "teletype": "comp_counters",
        "enduser_client_version": enduser_client_version,
    });
    info!("completion telemetry save \"{}\"", fn_comp.to_str().unwrap());
    let io_result = utils::file_save(fn_comp.clone(), big_json).await;
    if io_result.is_err() {
        error!("error: {}", io_result.err().unwrap());
    }
}


fn compress_into_counters(data: &Vec<TeleCompletionAccum>) -> Vec<TeleCompletionCounters> {
    let mut unique_combinations: HashMap<(String, String, bool), Vec<&TeleCompletionAccum>> = HashMap::new();

    for accum in data {
        let key = (accum.file_extension.clone(), accum.model.clone(), accum.multiline);
        unique_combinations.entry(key).or_default().push(accum);
    }

    let mut counters_vec: Vec<TeleCompletionCounters> = Vec::new();
    for (key, entries) in unique_combinations {
        let mut counters = TeleCompletionCounters::new(
            key.0.clone(),
            key.1.clone(),
            key.2
        );
        for entry in entries {
            if entry.finished_ts == 0 {
                continue;
            }
            update_counters(&mut counters, entry);
        }
        counters_vec.push(counters);
    }
    counters_vec
}

fn update_counters(counters: &mut TeleCompletionCounters, entry: &TeleCompletionAccum) {
    // Update counters based on entry values
    update_remaining_counters(entry.after_30s_remaining, &mut counters.after_30s_remaining_0, &mut counters.after_30s_remaining_0_50, &mut counters.after_30s_remaining_50_80, &mut counters.after_30s_remaining_80_100, &mut counters.after_30s_remaining_100);
    update_remaining_counters(entry.after_90s_remaining, &mut counters.after_90s_remaining_0, &mut counters.after_90s_remaining_0_50, &mut counters.after_90s_remaining_50_80, &mut counters.after_90s_remaining_80_100, &mut counters.after_90s_remaining_100);
    update_remaining_counters(entry.after_180s_remaining, &mut counters.after_180s_remaining_0, &mut counters.after_180s_remaining_0_50, &mut counters.after_180s_remaining_50_80, &mut counters.after_180s_remaining_80_100, &mut counters.after_180s_remaining_100);
    update_remaining_counters(entry.after_360s_remaining, &mut counters.after_360s_remaining_0, &mut counters.after_360s_remaining_0_50, &mut counters.after_360s_remaining_50_80, &mut counters.after_360s_remaining_80_100, &mut counters.after_360s_remaining_100);
}


fn update_remaining_counters(value: f64, counter_0: &mut i32, counter_0_50: &mut i32, counter_50_80: &mut i32, counter_80_100: &mut i32, counter_100: &mut i32) {
    if value == -1. { // default value
        return;
    }
    if value == 0. {
        *counter_0 += 1;
    } else if value <= 0.5 {
        *counter_0_50 += 1;
    } else if value <= 0.8 {
        *counter_50_80 += 1;
    } else if value < 1. {
        *counter_80_100 += 1;
    } else if value == 1. {
        *counter_100 += 1;
    } else {}
}


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct TeleCompletionCounters {
    // This struct is for serialization of the finalized counters
    file_extension: String,
    model: String,
    multiline: bool,

    after_30s_remaining_0: i32,
    after_30s_remaining_0_50: i32,
    after_30s_remaining_50_80: i32,
    after_30s_remaining_80_100: i32,
    after_30s_remaining_100: i32,

    after_90s_remaining_0: i32,
    after_90s_remaining_0_50: i32,
    after_90s_remaining_50_80: i32,
    after_90s_remaining_80_100: i32,
    after_90s_remaining_100: i32,

    after_180s_remaining_0: i32,
    after_180s_remaining_0_50: i32,
    after_180s_remaining_50_80: i32,
    after_180s_remaining_80_100: i32,
    after_180s_remaining_100: i32,

    after_360s_remaining_0: i32,
    after_360s_remaining_0_50: i32,
    after_360s_remaining_50_80: i32,
    after_360s_remaining_80_100: i32,
    after_360s_remaining_100: i32,
}

impl TeleCompletionCounters {
    fn new(
        file_extension: String, model: String, multiline: bool
    ) -> Self {
        Self {
            file_extension,
            model,
            multiline,

            after_30s_remaining_0: 0,
            after_30s_remaining_0_50: 0,
            after_30s_remaining_50_80: 0,
            after_30s_remaining_80_100: 0,
            after_30s_remaining_100: 0,

            after_90s_remaining_0: 0,
            after_90s_remaining_0_50: 0,
            after_90s_remaining_50_80: 0,
            after_90s_remaining_80_100: 0,
            after_90s_remaining_100: 0,

            after_180s_remaining_0: 0,
            after_180s_remaining_0_50: 0,
            after_180s_remaining_50_80: 0,
            after_180s_remaining_80_100: 0,
            after_180s_remaining_100: 0,

            after_360s_remaining_0: 0,
            after_360s_remaining_0_50: 0,
            after_360s_remaining_50_80: 0,
            after_360s_remaining_80_100: 0,
            after_360s_remaining_100: 0,
        }
    }
}
