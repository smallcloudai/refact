use std::collections::HashMap;
use tracing::{info};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use regex::Regex;
use serde::{Deserialize, Serialize};

use tokio::sync::RwLock as ARwLock;

use crate::global_context;
use crate::telemetry::utils;
use crate::telemetry::telemetry_structs::{SnippetTracker, Storage, TeleRobotHumanAccum};
use crate::telemetry::utils::compress_tele_records_to_file;


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
    let (added_characters_first_line, _) = utils::get_add_del_chars_from_texts(
        &removed_characters.lines().last().unwrap_or("").to_string(),
        &added_characters.lines().next().unwrap_or("").to_string(),
    );
    // info!(added_characters_first_line);
    let added_characters= vec![
        added_characters_first_line,
        added_characters.lines().skip(1).map(|x|x.to_string()).collect::<Vec<String>>().join("\n")
    ].join("\n");
    // info!(added_characters);
    let human_characters = re.replace_all(&added_characters, "").len() as i64 - rec.robot_characters_acc_baseline;
    info!("human_characters: +{}; robot_characters: +{}", 0.max(human_characters), rec.robot_characters_acc_baseline);
    rec.human_characters += 0.max(human_characters);
    rec.robot_characters += rec.robot_characters_acc_baseline;
    rec.robot_characters_acc_baseline = 0;
}


pub fn increase_counters_from_finished_snippet(
    storage_locked: &mut RwLockWriteGuard<Storage>,
    uri: &String,
    text: &String,
    snip: &SnippetTracker,
) {
    // info!("snip grey_text: {}", snip.grey_text);
    let now = chrono::Local::now().timestamp();
    if let Some(rec) = storage_locked.tele_robot_human.iter_mut().find(|stat| stat.uri.eq(uri)) {
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
    storage_locked.last_seen_file_texts.remove(text);
}

pub fn force_update_text_leap_calculations(
    tele_robot_human: &mut Vec<TeleRobotHumanAccum>,
    uri: &String,
    text: &String,
) {
    let now = chrono::Local::now().timestamp();
    if let Some(rec) = tele_robot_human.iter_mut().find(|stat| stat.uri.eq(uri)) {
        basetext_to_text_leap_calculations(rec, rec.baseline_text.clone(), text);
        rec.baseline_updated_ts = now;
        rec.baseline_text = text.clone();
    }
}

fn compress_robot_human(
    storage_locked: &RwLockReadGuard<Storage>
) -> Vec<TeleRobotHuman> {
    let mut unique_combinations: HashMap<(String, String), Vec<TeleRobotHumanAccum>> = HashMap::new();

    for accum in storage_locked.tele_robot_human.clone() {
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
    let last_seen_file_texts = cx.read().await.telemetry.read().unwrap().last_seen_file_texts.clone();
    for (k, v) in &last_seen_file_texts {
        force_update_text_leap_calculations(&mut cx.read().await.telemetry.write().unwrap().tele_robot_human, k, v);
    }
    let mut records = vec![];
    for rec in compress_robot_human(&cx.read().await.telemetry.read().unwrap()) {
        if rec.model.is_empty() && rec.robot_characters == 0 && rec.human_characters == 0 {
            continue;
        }
        let json_dict = serde_json::to_value(rec).unwrap();
        records.push(json_dict);
    }
    match compress_tele_records_to_file(cx.clone(), records, "robot_human".to_string(), "rh".to_string()).await {
        Ok(_) => {
            cx.write().await.telemetry.write().unwrap().tele_robot_human.clear();
        },
        Err(_) => {}
    };
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