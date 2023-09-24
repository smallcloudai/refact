use tokio::io::AsyncReadExt;
use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use tokio::sync::RwLock as ARwLock;
use serde_json::json;

use crate::caps::CodeAssistantCaps;
use crate::global_context;
use crate::telemetry_basic;
use crate::telemetry_correction;

const TELEMETRY_COMPRESSION_SECONDS: u64 = 3600;


pub struct Storage {
    pub last_flushed_ts: i64,
    pub tele_net: Vec<telemetry_basic::TelemetryNetwork>,
    pub tele_completion: Vec<telemetry_basic::TelemetryCompletion>,
    pub tele_snippets: Vec<telemetry_correction::SnippetTelemetry>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            last_flushed_ts: chrono::Local::now().timestamp(),
            tele_net: Vec::new(),
            tele_completion: Vec::new(),
            tele_snippets: Vec::new(),
        }
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
        info!("sending telemetry file\n{}\nto url\n{}", path.to_str().unwrap(), telemetry_basic_dest);
        let resp_maybe = http_client.post(telemetry_basic_dest.clone())
           .body(contents)
           .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", api_key))
           .header(reqwest::header::CONTENT_TYPE, format!("application/json"))
           .send().await;
        if resp_maybe.is_err() {
            error!("telemetry send failed: {}\ndest url was\n{}", resp_maybe.err().unwrap(), telemetry_basic_dest);
            break;
        }
        let resp = resp_maybe.unwrap();
        if resp.status()!= reqwest::StatusCode::OK {
            error!("telemetry send failed: {}\ndest url was\n{}", resp.status(), telemetry_basic_dest);
            break;
        }
        let resp_body = resp.text().await.unwrap_or_else(|_| "-empty-".to_string());
        info!("telemetry send success, response:\n{}", resp_body);
        let resp_json = serde_json::from_str::<serde_json::Value>(&resp_body).unwrap_or_else(|_| json!({}));
        let retcode = resp_json["retcode"].as_str().unwrap_or("").to_string();
        if retcode != "OK" {
            error!("retcode is not OK");
            break;
        }
        let new_path = dir_sent.join(path.file_name().unwrap());
        info!("success, moving file to {}", new_path.to_str().unwrap());
        let res = tokio::fs::rename(path, new_path).await;
        if res.is_err() {
            error!("telemetry send success, but cannot move file: {}", res.err().unwrap());
            error!("pretty bad, because this can lead to infinite sending of the same file");
            break;
        }
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
    telemetry_basic::compress_basic_telemetry_to_file(global_context.clone()).await;
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
