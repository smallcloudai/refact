use std::sync::Arc;
use std::collections::HashMap;
use serde_json::json;

use tokio::sync::RwLock as ARwLock;

use crate::global_context;
use crate::telemetry::utils::compress_tele_records_to_file;

pub async fn compress_basic_telemetry_to_file(
    cx: Arc<ARwLock<global_context::GlobalContext>>,
) {
    let mut key2cnt = HashMap::new();
    let mut key2dict = HashMap::new();

    for rec in cx.read().await.telemetry.read().unwrap().tele_net.iter() {
        let key = format!("{}/{}/{}/{}", rec.url, rec.scope, rec.success, rec.error_message);
        if !key2dict.contains_key(&key) {
            key2dict.insert(key.clone(), serde_json::to_value(rec).unwrap());
            key2cnt.insert(key.clone(), 0);
        }
        key2cnt.insert(key.clone(), key2cnt[&key] + 1);
    }

    let mut records = vec![];
    for (key, cnt) in key2cnt.iter() {
        let mut json_dict = key2dict[key.as_str()].clone();
        json_dict["counter"] = json!(cnt);
        records.push(json_dict);
    }
    match compress_tele_records_to_file(cx.clone(), records, "network".to_string(), "net".to_string()).await {
        Ok(_) => {
            cx.write().await.telemetry.write().unwrap().tele_net.clear();
        },
        Err(_) => {}
    };
}
