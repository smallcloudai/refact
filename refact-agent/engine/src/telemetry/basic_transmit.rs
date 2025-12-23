use tracing::{error, info};
use std::sync::Arc;
use std::path::PathBuf;
use serde_json::json;

use tokio::sync::RwLock as ARwLock;
use crate::caps::CodeAssistantCaps;

use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::telemetry::{basic_chat, basic_network};
use crate::telemetry::basic_robot_human;
use crate::telemetry::basic_comp_counters;
use crate::telemetry::utils::{sorted_json_files, read_file, cleanup_old_files, telemetry_storage_dirs};


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
        .header(reqwest::header::CONTENT_TYPE, "application/json".to_string())
        .send().await;
    if resp_maybe.is_err() {
        return Err(format!("telemetry send failed: {}\ndest url was\n{}", resp_maybe.err().unwrap(), telemetry_dest));
    }
    let resp = resp_maybe.unwrap();
    if resp.status() != reqwest::StatusCode::OK {
        return Err(format!("telemetry send failed: {}\ndest url was\n{}", resp.status(), telemetry_dest));
    }
    let resp_body = resp.text().await.unwrap_or_else(|_| "-empty-".to_string());
    // info!("telemetry send success, response:\n{}", resp_body);
    let resp_json = serde_json::from_str::<serde_json::Value>(&resp_body).unwrap_or_else(|_| json!({}));
    let retcode = resp_json["retcode"].as_str().unwrap_or("").to_string();
    if retcode != "OK" {
        return Err("retcode is not OK".to_string());
    } else {
        info!("telemetry send success");
    }
    Ok(())
}

const TELEMETRY_FILES_SUFFIXES: [&str; 4] = ["-chat.json", "-net.json", "-rh.json", "-comp.json"];

pub async fn send_telemetry_files_to_mothership(
    dir_compressed: PathBuf,
    dir_sent: PathBuf,
    telemetry_basic_dest: String,
    api_key: String,
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    // Send files found in dir_compressed, move to dir_sent if successful.
    let files = sorted_json_files(dir_compressed.clone()).await;
    let file_prefix = {
        let cx = gcx.read().await;
        cx.cmdline.get_prefix()
    };

    for path in files {
        let contents_maybe = read_file(path.clone()).await;
        if contents_maybe.is_err() {
            error!("cannot read {}: {}", path.display(), contents_maybe.err().unwrap());
            continue;
        }
        let contents = contents_maybe.unwrap();
        let path_str = path.to_str().unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        if filename.starts_with(&file_prefix) && TELEMETRY_FILES_SUFFIXES.iter().any(|s| path_str.ends_with(s)) {
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
        // info!("success, moving file to {}", new_path.to_str().unwrap());
        let res = tokio::fs::rename(path, new_path).await;
        if res.is_err() {
            error!("telemetry send success, but cannot move file: {}", res.err().unwrap());
            error!("pretty bad, because this can lead to infinite sending of the same file");
            break;
        }
    }
}

pub async fn basic_telemetry_compress(
    global_context: Arc<ARwLock<GlobalContext>>,
) {
    info!("basic telemetry compression starts");
    basic_network::compress_basic_telemetry_to_file(global_context.clone()).await;
    basic_chat::compress_basic_chat_telemetry_to_file(global_context.clone()).await;
    basic_robot_human::tele_robot_human_compress_to_file(global_context.clone()).await;
    basic_comp_counters::compress_tele_completion_to_file(global_context.clone()).await;
}

pub async fn basic_telemetry_send(
    global_context: Arc<ARwLock<GlobalContext>>,
    caps: Arc<CodeAssistantCaps>,
) -> () {
    let (cache_dir, api_key, enable_basic_telemetry) = {
        let cx = global_context.write().await;
        (
            cx.cache_dir.clone(),
            cx.cmdline.api_key.clone(),
            cx.cmdline.basic_telemetry.clone(),
        )
    };
    let (dir_compressed, dir_sent) = telemetry_storage_dirs(&cache_dir).await;

    if enable_basic_telemetry && !caps.telemetry_basic_dest.is_empty() {
        send_telemetry_files_to_mothership(
            dir_compressed.clone(),
            dir_sent.clone(),
            caps.telemetry_basic_dest.clone(),
            api_key,
            global_context.clone()
        ).await;
    } else {
        if !enable_basic_telemetry {
            info!("basic telemetry sending not enabled, skip");
        }
        if caps.telemetry_basic_dest.is_empty() {
            info!("basic telemetry dest is empty, skip");
        }
    }
    cleanup_old_files(dir_compressed, TELEMETRY_FILES_KEEP).await;
    cleanup_old_files(dir_sent, TELEMETRY_FILES_KEEP).await;
}

pub async fn telemetry_background_task(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    loop {
        match try_load_caps_quickly_if_not_present(global_context.clone(), 0).await {
            Ok(caps) => {
                basic_telemetry_compress(global_context.clone()).await;
                basic_telemetry_send(global_context.clone(), caps.clone()).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(TELEMETRY_TRANSMIT_EACH_N_SECONDS)).await;
            },
            Err(e) => {
                error!("telemetry send failed: no caps, trying again in {}, error: {}", TELEMETRY_TRANSMIT_EACH_N_SECONDS, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(TELEMETRY_TRANSMIT_EACH_N_SECONDS)).await;
            }
        };
    }
}
