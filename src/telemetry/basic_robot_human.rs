use std::collections::HashMap;
use tracing::{debug, error, info};
use std::sync::{Arc, RwLockWriteGuard};
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use tokio::sync::RwLock as ARwLock;

use crate::global_context;
use crate::telemetry::utils;
use crate::telemetry::telemetry_structs;
use crate::telemetry::telemetry_structs::{SnippetTracker, TeleRobotHumanAccum};



pub fn create_robot_human_record_if_not_exists(
    tele_robot_human: &mut Vec<TeleRobotHumanAccum>,
    uri: &String,
    text: &String
) {
    let record_mb = tele_robot_human.iter_mut().find(|stat| stat.uri.eq(uri));
    if record_mb.is_some() {
        return;
    }
    info!("create_robot_human_rec_if_not_exists: new uri {}", uri);
    let record = TeleRobotHumanAccum::new(
        uri.clone(),
        text.clone(),
    );
    tele_robot_human.push(record);
}


fn update_robot_characters_baseline(
    rec: &mut TeleRobotHumanAccum,
    snip: &SnippetTracker
) {
    let re = Regex::new(r"\s+").unwrap();
    let robot_characters = re.replace_all(&snip.grey_text, "").len() as i64;
    rec.robot_characters_acc_baseline += robot_characters;
}

fn basetext_to_text_leap_calculations(
    rec: &mut TeleRobotHumanAccum,
    baseline_text: String,
    text: &String,
) {
    let re = Regex::new(r"\s+").unwrap();
    let (added_characters, removed_characters) = utils::get_add_del_from_texts(&baseline_text, text);

    let (added_characters, _) = utils::get_add_del_chars_from_texts(&removed_characters, &added_characters);

    // let real_characters_added = re.replace_all(&added_characters, "").len() as i64 - re.replace_all(&removed_characters, "").len() as i64;
    let human_characters = re.replace_all(&added_characters, "").len() as i64 - rec.robot_characters_acc_baseline;
    debug!("human_characters: +{}; robot_characters: +{}", human_characters, rec.robot_characters_acc_baseline);
    rec.human_characters += human_characters;
    rec.robot_characters += rec.robot_characters_acc_baseline;
    rec.robot_characters_acc_baseline = 0;
}


pub fn increase_counters_from_finished_snippet(
    tele_robot_human: &mut Vec<TeleRobotHumanAccum>,
    uri: &String,
    text: &String,
    snip: &SnippetTracker,
) {
    info!("snip grey_text: {}", snip.grey_text);
    let now = chrono::Local::now().timestamp();
    if let Some(rec) = tele_robot_human.iter_mut().find(|stat| stat.uri.eq(uri)) {
        if rec.used_snip_ids.contains(&snip.snippet_telemetry_id) {
            return;
        }

        if rec.used_snip_ids.is_empty() {
            rec.model = snip.model.clone();
        }

        update_robot_characters_baseline(rec, snip);
        basetext_to_text_leap_calculations(rec, rec.baseline_text.clone(), text);

        rec.used_snip_ids.push(snip.snippet_telemetry_id);
        rec.baseline_updated_ts = now;
        rec.baseline_text = text.clone();
    }
}

fn compress_robot_human(
    storage_locked: &mut RwLockWriteGuard<telemetry_structs::Storage>
) -> Vec<TeleRobotHuman> {
    let mut unique_combinations: HashMap<(String, String), Vec<TeleRobotHumanAccum>> = HashMap::new();

    let tele_robot_human = storage_locked.tele_robot_human.clone();

    for accum in tele_robot_human {
        let key = (accum.file_extension.clone(), accum.model.clone());
        unique_combinations.entry(key).or_default().push(accum);
    }
    let mut compressed_vec= vec![];
    for (key, entries) in unique_combinations {
        // info!("compress_robot_human: compressing {} entries for key {:?}", entries.len(), key);
        let mut record = TeleRobotHuman::new(
            key.0.clone(),
            key.1.clone()
        );
        for entry in entries {
            record.human_characters += entry.human_characters;
            record.robot_characters += entry.robot_characters + entry.robot_characters_acc_baseline;
            record.completions_cnt += entry.used_snip_ids.len() as i64;
        }
        compressed_vec.push(record);
    }
    compressed_vec
}

pub async fn tele_robot_human_compress_to_file(
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
        for rec in compress_robot_human(&mut storage_locked) {
            if rec.model.is_empty() && rec.robot_characters == 0 && rec.human_characters == 0 {
                continue;
            }
            let json_dict = serde_json::to_value(rec).unwrap();
            records.as_array_mut().unwrap().push(json_dict);
        }
        storage_locked.tele_robot_human.clear();
    }
    if records.as_array().unwrap().is_empty() {
        info!("no robot_human telemetry to save");
        return;
    }
    let (dir, _) = utils::telemetry_storage_dirs(&cache_dir).await;

    let fn_rh = dir.join(format!("{}-rh.json", now.format("%Y%m%d-%H%M%S")));
    let big_json_rh = json!({
        "records": records,
        "ts_start": now.timestamp(),
        "ts_end": now.timestamp(),
        "teletype": "robot_human",
        "enduser_client_version": enduser_client_version,
    });
    info!("robot_human telemetry save \"{}\"", fn_rh.to_str().unwrap());
    let io_result = utils::file_save(fn_rh.clone(), big_json_rh).await;
    if io_result.is_err() {
        error!("error: {}", io_result.err().unwrap());
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct TeleRobotHuman {
    file_extension: String,
    model: String,

    human_characters: i64,
    robot_characters: i64,
    completions_cnt: i64,
}

impl TeleRobotHuman {
    fn new(
        file_extension: String, model: String
    ) -> Self {
        Self {
            file_extension,
            model,

            human_characters: 0,
            robot_characters: 0,
            completions_cnt: 0
        }
    }
}