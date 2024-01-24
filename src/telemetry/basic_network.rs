use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use std::collections::HashMap;
use serde_json::json;

use tokio::sync::RwLock as ARwLock;

use crate::global_context;
use crate::telemetry::utils::{file_save, telemetry_storage_dirs};
use crate::telemetry::telemetry_structs;

fn _key_telemetry_network(rec: &telemetry_structs::TelemetryNetwork) -> String {
    format!("{}/{}/{}/{}", rec.url, rec.scope, rec.success, rec.error_message)
}

fn compress_telemetry_network(
    storage: Arc<StdRwLock<telemetry_structs::Storage>>,
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
    let mut records = json!([]);
    for (key, cnt) in key2cnt.iter() {
        let mut json_dict = key2dict[key.as_str()].clone();
        json_dict["counter"] = json!(cnt);
        records.as_array_mut().unwrap().push(json_dict);
    }
    records
}

pub async fn compress_basic_telemetry_to_file(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    let now = chrono::Local::now();
    let cache_dir: PathBuf;
    let storage: Arc<StdRwLock<telemetry_structs::Storage>>;
    let enduser_client_version;
    let file_prefix;
    {
        let cx_locked = cx.read().await;
        storage = cx_locked.telemetry.clone();
        cache_dir = cx_locked.cache_dir.clone();
        enduser_client_version = cx_locked.cmdline.enduser_client_version.clone();
        file_prefix = cx_locked.cmdline.get_prefix();
    }
    let (dir, _) = telemetry_storage_dirs(&cache_dir).await;

    let records = compress_telemetry_network(storage.clone());
    let fn_net = dir.join(format!("{}-{}-net.json", file_prefix, now.format("%Y%m%d-%H%M%S")));
    let mut big_json_net = json!({
        "records": records,
        "ts_end": now.timestamp(),
        "teletype": "network",
        "enduser_client_version": enduser_client_version,
    });
    { // clear
        let mut storage_locked = storage.write().unwrap();
        storage_locked.tele_net.clear();
        big_json_net.as_object_mut().unwrap().insert("ts_start".to_string(), json!(storage_locked.last_flushed_ts));
        storage_locked.last_flushed_ts = now.timestamp();
    }
    if records.as_array().unwrap().is_empty() {
        info!("no network telemetry to save");
        return;
    }
    // even if there's an error with i/o, storage is now clear, preventing infinite memory growth
    info!("basic telemetry save \"{}\"", fn_net.to_str().unwrap());
    let io_result = file_save(fn_net.clone(), big_json_net).await;
    if io_result.is_err() {
        error!("error: {}", io_result.err().unwrap());
    }
}
