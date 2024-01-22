use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::path::PathBuf;
use serde_json::json;

use tokio::sync::RwLock as ARwLock;

use crate::caps::CodeAssistantCaps;
use crate::global_context;
use crate::global_context::GlobalContext;
use crate::telemetry::basic_network;
use crate::telemetry::basic_robot_human;
use crate::telemetry::basic_comp_counters;
use crate::telemetry::utils::{sorted_json_files, read_file, cleanup_old_files, telemetry_storage_dirs};


const TELEMETRY_TRANSMIT_AFTER_START_SECONDS: u64 = 60;
const TELEMETRY_TRANSMIT_EACH_N_SECONDS: u64 = 3600;
const TELEMETRY_FILES_KEEP: i32 = 128;


pub async fn send_telemetry_data(
    contents: String,
    telemetry_dest: &String,
    api_key: &String,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String>{
    let http_client = gcx.read().await.http_client.clone();
    let resp_maybe = http_client.post(telemetry_dest.clone())
        .body(contents)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", api_key))
        .header(reqwest::header::CONTENT_TYPE, format!("application/json"))
        .send().await;
    if resp_maybe.is_err() {
        return Err(format!("telemetry send failed: {}\ndest url was\n{}", resp_maybe.err().unwrap(), telemetry_dest));
    }
    let resp = resp_maybe.unwrap();
    if resp.status()!= reqwest::StatusCode::OK {
        return Err(format!("telemetry send failed: {}\ndest url was\n{}", resp.status(), telemetry_dest));
    }
    let resp_body = resp.text().await.unwrap_or_else(|_| "-empty-".to_string());
    info!("telemetry send success, response:\n{}", resp_body);
    let resp_json = serde_json::from_str::<serde_json::Value>(&resp_body).unwrap_or_else(|_| json!({}));
    let retcode = resp_json["retcode"].as_str().unwrap_or("").to_string();
    if retcode != "OK" {
        return Err("retcode is not OK".to_string());
    }
    Ok(())
}

pub async fn send_telemetry_files_to_mothership(
    dir_compressed: PathBuf,
    dir_sent: PathBuf,
    telemetry_basic_dest: String,
    api_key: String,
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    // Send files found in dir_compressed, move to dir_sent if successful.
    let files = sorted_json_files(dir_compressed.clone()).await;
    for path in files {
        let contents_maybe = read_file(path.clone()).await;
        if contents_maybe.is_err() {
            error!("cannot read {}: {}", path.display(), contents_maybe.err().unwrap());
            continue
        }
        let contents = contents_maybe.unwrap();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with("-net.json") || path_str.ends_with("-rh.json") || path_str.ends_with("-comp.json") {
            info!("sending telemetry file\n{}\nto url\n{}", path.to_str().unwrap(), telemetry_basic_dest);
            let resp = send_telemetry_data(contents, &telemetry_basic_dest,
                                           &api_key, gcx.clone()).await;
            if resp.is_err() {
                error!("telemetry send failed: {}", resp.err().unwrap());
                continue;
            }
        } else {
            continue;
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

pub async fn basic_telemetry_compress(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) {
    info!("basic telemetry compression starts");
    basic_network::compress_basic_telemetry_to_file(global_context.clone()).await;
    basic_robot_human::tele_robot_human_compress_to_file(global_context.clone()).await;
    basic_comp_counters::compress_tele_completion_to_file(global_context.clone()).await;
}

pub async fn basic_telemetry_send(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    let caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>;
    let api_key: String;
    let enable_basic_telemetry: bool;   // from command line, will not send anything if false
    let mut telemetry_basic_dest: String = String::new();
    let cache_dir: PathBuf;
    {
        let cx = global_context.write().await;
        caps = cx.caps.clone();
        cache_dir = cx.cache_dir.clone();
        api_key = cx.cmdline.api_key.clone();
        enable_basic_telemetry = cx.cmdline.basic_telemetry;
    }
    let (dir_compressed, dir_sent) = telemetry_storage_dirs(&cache_dir).await;

    if caps.is_some() {
        telemetry_basic_dest = caps.clone().unwrap().read().unwrap().telemetry_basic_dest.clone();
    }

    if enable_basic_telemetry && !telemetry_basic_dest.is_empty() {
        send_telemetry_files_to_mothership(
            dir_compressed.clone(),
            dir_sent.clone(),
            telemetry_basic_dest.clone(),
            api_key,
            global_context.clone()
        ).await;
    } else {
        if !enable_basic_telemetry {
            info!("basic telemetry sending not enabled, skip");
        }
        if telemetry_basic_dest.is_empty() {
            info!("basic telemetry dest is empty, skip");
        }
    }
    cleanup_old_files(dir_compressed, TELEMETRY_FILES_KEEP).await;
    cleanup_old_files(dir_sent, TELEMETRY_FILES_KEEP).await;
}

pub async fn telemetry_background_task(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    tokio::time::sleep(tokio::time::Duration::from_secs(TELEMETRY_TRANSMIT_AFTER_START_SECONDS)).await;
    basic_telemetry_send(global_context.clone()).await;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(TELEMETRY_TRANSMIT_EACH_N_SECONDS)).await;
        basic_telemetry_compress(global_context.clone()).await;
        basic_telemetry_send(global_context.clone()).await;
    }
}
