use tokio::io::AsyncReadExt;
use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock as ARwLock;
use serde_json::json;
use serde::{Deserialize, Serialize};

use crate::caps::CodeAssistantCaps;
use crate::global_context;


const TELEMETRY_COMPRESSION_SECONDS: u64 = 3600;


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
pub struct TelemetryCompletion {
    pub language: String,
    pub multiline: bool,
    pub accepted: bool,
    pub user_pondered_600ms: bool,
    pub user_pondered_1200ms: bool,
    // -- key above, calculate average below --
    pub generated_chars: usize,
    pub remaining_percent: f64,
}


pub struct Storage {
    pub last_flushed_ts: i64,
    pub tele_net: Vec<TelemetryNetwork>,
    pub tele_comp: Vec<TelemetryCompletion>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            last_flushed_ts: chrono::Local::now().timestamp(),
            tele_net: Vec::new(),
            tele_comp: Vec::new(),
        }
    }
}

fn _key_telemetry_network(rec: &TelemetryNetwork) -> String {
    format!("{}/{}/{}/{}", rec.url, rec.scope, rec.success, rec.error_message)
}

fn _compress_telemetry_network(
    storage: Arc<StdRwLock<Storage>>,
) -> serde_json::Value {
    let mut key2cnt = HashMap::<String, i32>::new();
    let mut key2dict = HashMap::<String, serde_json::Value>::new();
    {
        let storage_locked = storage.write().unwrap();
        for rec in storage_locked.tele_net.iter() {
            let key = _key_telemetry_network(rec);
            if !key2dict.contains_key(&key) {
                key2dict.insert(key.clone(), serde_json::to_value(rec).unwrap());
                key2cnt.insert(key.clone(), 0);
            }
            key2cnt.insert(key.clone(), key2cnt[&key] + 1);
        }
    }
    let mut records = serde_json::json!([]);
    for (key, cnt) in key2cnt.iter() {
        let mut json_dict = key2dict[key.as_str()].clone();
        json_dict["counter"] = json!(cnt);
        records.as_array_mut().unwrap().push(json_dict);
    }
    records
}

fn _key_telemetry_completion(rec: &TelemetryCompletion) -> String {
    format!("{}/{}/{}/{}/{}", rec.language, rec.multiline, rec.accepted, rec.user_pondered_600ms, rec.user_pondered_1200ms)
}

fn _compress_telemetry_completion(
    _storage: Arc<StdRwLock<Storage>>,
) -> serde_json::Value {
    unimplemented!();
}

pub async fn compress_basic_telemetry_to_file(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    let now = chrono::Local::now();
    let cache_dir: PathBuf;
    let storage: Arc<StdRwLock<Storage>>;
    let enduser_client_version;
    {
        let cx_locked = cx.read().await;
        storage = cx_locked.telemetry.clone();
        cache_dir = cx_locked.cache_dir.clone();
        enduser_client_version = cx_locked.cmdline.enduser_client_version.clone();
    }
    let dir = cache_dir.join("telemetry").join("compressed");
    tokio::fs::create_dir_all(dir.clone()).await.unwrap_or_else(|_| {});

    let records = _compress_telemetry_network(storage.clone());
    let fn_net = dir.join(format!("{}-net.json", now.format("%Y%m%d-%H%M%S")));
    let mut big_json_net = json!({});
    {
        let mut storage_locked = storage.write().unwrap();
        storage_locked.tele_net.clear();
        storage_locked.tele_comp.clear();
        big_json_net.as_object_mut().unwrap().insert("ts_start".to_string(), json!(storage_locked.last_flushed_ts));
        storage_locked.last_flushed_ts = now.timestamp();
    }
    big_json_net.as_object_mut().unwrap().insert("records".to_string(), records);
    big_json_net.as_object_mut().unwrap().insert("ts_end".to_string(), json!(now.timestamp()));
    big_json_net.as_object_mut().unwrap().insert("teletype".to_string(), json!("network"));
    big_json_net.as_object_mut().unwrap().insert("enduser_client_version".to_string(), json!(enduser_client_version));
    // even if there's an error with i/o, storage is now clear, preventing infinite memory growth
    info!("basic telemetry save \"{}\"", fn_net.to_str().unwrap());
    let mut f_net = tokio::fs::File::create(fn_net).await.unwrap();
    f_net.write_all(serde_json::to_string_pretty(&big_json_net).unwrap().as_bytes()).await.unwrap();
}

async fn _sorted_files(dir: PathBuf) -> Vec<PathBuf> {
    // Most recent files first
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        let mut sorted = Vec::<PathBuf>::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            if !entry.file_type().await.unwrap().is_file() {
                continue;
            }
            let path = entry.path();
            if !path.to_str().unwrap().ends_with(".json") {
                continue;
            }
            sorted.push(path);
        }
        sorted.sort_by(|a, b| b.cmp(&a));
        sorted
    } else {
        Vec::<PathBuf>::new()
    }
}

pub async fn cleanup_old_files(
    dir: PathBuf,
    how_much_to_keep: i32,
) {
    let files = _sorted_files(dir.clone()).await;
    let mut leave_alone = how_much_to_keep;
    for path in files {
        leave_alone -= 1;
        if leave_alone > 0 {
            // info!("leave_alone telemetry file: {}", path.to_str().unwrap());
            continue;
        }
        info!("removing old telemetry file: {}", path.to_str().unwrap());
        tokio::fs::remove_file(path).await.unwrap_or_else(|e| {
            error!("error removing old telemetry file: {}", e);
            // better to continue deleting, not much we can do
        });
    }
}

async fn _read_file(path: PathBuf) -> Result<String, String> {
    let mut f = tokio::fs::File::open(path.clone()).await.map_err(|e| format!("{:?}", e))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents).await.map_err(|e| format!("{}", e))?;
    Ok(contents)
}

pub async fn send_telemetry_files_to_mothership(
    dir_compressed: PathBuf,
    dir_sent: PathBuf,
    telemetry_basic_dest: String,
    api_key: String,
) {
    // Send files found in dir_compressed, move to dir_sent if successful.
    let files = _sorted_files(dir_compressed.clone()).await;
    let http_client = reqwest::Client::new();
    for path in files {
        let contents_maybe = _read_file(path.clone()).await;
        if contents_maybe.is_err() {
            error!("cannot read {}: {}", path.display(), contents_maybe.err().unwrap());
            break;
        }
        let contents = contents_maybe.unwrap();
        info!("sending telemetry file: {}", path.to_str().unwrap());
        let resp = http_client.post(telemetry_basic_dest.clone())
           .body(contents)
           .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", api_key))
           .header(reqwest::header::CONTENT_TYPE, format!("application/json"))
           .send().await.unwrap();
        if resp.status()!= reqwest::StatusCode::OK {
            error!("telemetry send failed: {}\ndest url was: {}", resp.status(), telemetry_basic_dest);
            break;
        }
        let new_path = dir_sent.join(path.file_name().unwrap());
        info!("success, moving telemetry file: {} to {}", path.to_str().unwrap(), new_path.to_str().unwrap());
        tokio::fs::rename(path, new_path).await.unwrap();
    }
}

pub async fn telemetry_full_cycle(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
    skip_sending_part: bool,
) -> () {
    info!("basic telemetry compression starts");
    let caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>;
    let api_key: String;
    let mothership_enabled: bool;
    let mut telemetry_basic_dest: String = String::new();
    let cache_dir: PathBuf;
    {
        let cx = global_context.write().await;
        caps = cx.caps.clone();
        cache_dir = cx.cache_dir.clone();
        api_key = cx.cmdline.api_key.clone();
        mothership_enabled = cx.cmdline.basic_telemetry;
    }
    if caps.is_some() {
        telemetry_basic_dest = caps.unwrap().read().unwrap().telemetry_basic_dest.clone();
    }
    compress_basic_telemetry_to_file(global_context.clone()).await;
    let dir_compressed = cache_dir.join("telemetry").join("compressed");
    let dir_sent = cache_dir.join("telemetry").join("sent");
    if mothership_enabled && !telemetry_basic_dest.is_empty() && !skip_sending_part {
        send_telemetry_files_to_mothership(dir_compressed.clone(), dir_sent.clone(), telemetry_basic_dest, api_key).await;
    }
    if !mothership_enabled {
        info!("telemetry sending not enabled, skip");
    }
    cleanup_old_files(dir_compressed, 10).await;
    cleanup_old_files(dir_sent, 10).await;
}

pub async fn telemetry_background_task(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(TELEMETRY_COMPRESSION_SECONDS)).await;
        telemetry_full_cycle(global_context.clone(), false).await;
    }
}
