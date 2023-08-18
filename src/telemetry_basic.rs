use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock as ARwLock;
use serde_json::json;
use serde::{Deserialize, Serialize};

use crate::global_context;
use crate::telemetry_storage;



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
    pub model: String,
    pub language: String,
    pub multiline: bool,
    pub accepted: bool,
    // -- key above, calculate below --
    pub counter: usize,
    pub generated_chars: usize,  // becomes float
    // sum(after_walkaway_*) == counter
    // "walkaway" means user has edited other code in a different spot
    pub after_walkaway_remaining00: usize,
    pub after_walkaway_remaining00to50: usize,
    pub after_walkaway_remaining50to95: usize,
    pub after_walkaway_remaining95to99: usize,
    pub after_walkaway_remaining100: usize,
    pub after_30s_remaining00: usize,    // 30s point is independent of user walked away or not
    pub after_30s_remaining00to50: usize,
    pub after_30s_remaining50to95: usize,
    pub after_30s_remaining95to99: usize,
    pub after_30s_remaining100: usize,
    pub after_300s_remaining00: usize,
    pub after_300s_remaining00to50: usize,
    pub after_300s_remaining50to95: usize,
    pub after_300s_remaining95to99: usize,
    pub after_300s_remaining100: usize,
}


fn _key_telemetry_network(rec: &TelemetryNetwork) -> String {
    format!("{}/{}/{}/{}", rec.url, rec.scope, rec.success, rec.error_message)
}

fn _compress_telemetry_network(
    storage: Arc<StdRwLock<telemetry_storage::Storage>>,
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

// fn _key_telemetry_completion(rec: &TelemetryCompletion) -> String {
//     format!("{}/{}/{}/{}/{}", rec.language, rec.multiline, rec.accepted, rec.user_pondered_600ms, rec.user_pondered_1200ms)
// }

fn _compress_telemetry_completion(
    _storage: Arc<StdRwLock<telemetry_storage::Storage>>,
) -> serde_json::Value {
    unimplemented!();
}

pub async fn compress_basic_telemetry_to_file(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    let now = chrono::Local::now();
    let cache_dir: PathBuf;
    let storage: Arc<StdRwLock<telemetry_storage::Storage>>;
    let enduser_client_version;
    {
        let cx_locked = cx.read().await;
        storage = cx_locked.telemetry.clone();
        cache_dir = cx_locked.cache_dir.clone();
        enduser_client_version = cx_locked.cmdline.enduser_client_version.clone();
    }
    let dir = cache_dir.join("telemetry").join("compressed");
    tokio::fs::create_dir_all(dir.clone()).await.unwrap_or_else(|_| {});
    let dir2 = cache_dir.join("telemetry").join("sent");  // while we're at it ...
    tokio::fs::create_dir_all(dir2.clone()).await.unwrap_or_else(|_| {});

    let records = _compress_telemetry_network(storage.clone());
    let fn_net = dir.join(format!("{}-net.json", now.format("%Y%m%d-%H%M%S")));
    let mut big_json_net = json!({
        "records": records,
        "ts_end": now.timestamp(),
        "teletype": "network",
        "enduser_client_version": enduser_client_version,
    });
    { // clear
        let mut storage_locked = storage.write().unwrap();
        storage_locked.tele_net.clear();
        storage_locked.tele_completion.clear();
        big_json_net.as_object_mut().unwrap().insert("ts_start".to_string(), json!(storage_locked.last_flushed_ts));
        storage_locked.last_flushed_ts = now.timestamp();
    }
    // even if there's an error with i/o, storage is now clear, preventing infinite memory growth
    info!("basic telemetry save \"{}\"", fn_net.to_str().unwrap());
    let io_result = file_save(fn_net.clone(), big_json_net).await;
    if io_result.is_err() {
        error!("error: {}", io_result.err().unwrap());
    }
}

pub async fn file_save(path: PathBuf, json: serde_json::Value) -> Result<(), String> {
    let mut f = tokio::fs::File::create(path).await.map_err(|e| format!("{:?}", e))?;
    f.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes()).await.map_err(|e| format!("{}", e))?;
    Ok(())
}
