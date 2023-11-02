use tracing::{error, info};
use std::sync::Arc;
use serde_json::json;

use tokio::sync::RwLock as ARwLock;

use crate::telemetry::telemetry_structs::SnippetTracker;
use crate::telemetry::basic_transmit;
use crate::global_context;


const SNIP_NOT_ACCEPTED_TIMEOUT_AFTER : i64 = 30;
const SNIP_ACCEPTED_NOT_FINISHED_TIMEOUT_AFTER: i64 = 600;


pub async fn send_finished_snippets(gcx: Arc<ARwLock<global_context::GlobalContext>>) {
    let tele_storage;
    let now = chrono::Local::now().timestamp();
    let enduser_client_version;
    let api_key: String;
    let caps: Option<Arc<std::sync::RwLock<crate::caps::CodeAssistantCaps>>>;
    let enable_snippet_telemetry: bool;  // from command line, will not send anything if false
    let mut telemetry_corrected_snippets_dest = String::new();
    {
        let cx = gcx.read().await;
        enduser_client_version = cx.cmdline.enduser_client_version.clone();
        tele_storage = cx.telemetry.clone();
        api_key = cx.cmdline.api_key.clone();
        caps = cx.caps.clone();
        enable_snippet_telemetry = cx.cmdline.snippet_telemetry;
    }
    if let Some(caps) = &caps {
        telemetry_corrected_snippets_dest = caps.read().unwrap().telemetry_corrected_snippets_dest.clone();
    }

    let mut snips_send: Vec<SnippetTracker> = vec![];
    {
        let mut to_remove: Vec<usize> = vec![];
        let mut storage_locked = tele_storage.write().unwrap();
        for (idx, snip) in &mut storage_locked.tele_snippets.iter().enumerate() {
            if snip.accepted_ts != 0 {
                if snip.finished_ts != 0 {
                    to_remove.push(idx);
                    snips_send.push(snip.clone());
                } else if snip.created_ts + SNIP_ACCEPTED_NOT_FINISHED_TIMEOUT_AFTER < now {
                    to_remove.push(idx)
                }
                continue;
            }
            if snip.accepted_ts == 0 && snip.created_ts + SNIP_NOT_ACCEPTED_TIMEOUT_AFTER < now {
                to_remove.push(idx);
                continue;
            }
        }
        for idx in to_remove.iter().rev() {
            storage_locked.tele_snippets.remove(*idx);
        }
    }

    if !enable_snippet_telemetry {
        return;
    }
    if telemetry_corrected_snippets_dest.is_empty() {
        return;
    }
    if snips_send.is_empty() {
        return;
    }
    info!("sending {} snippets", snips_send.len());

    for snip in snips_send {
        let json_dict = serde_json::to_value(snip).unwrap();
        info!("sending snippet: {:?}", json_dict);
        let big_json_snip = json!({
            "records": [json_dict],
            "ts_start": now,
            "ts_end": chrono::Local::now().timestamp(),
            "teletype": "snippets",
            "enduser_client_version": enduser_client_version,
        });
        let resp_maybe = basic_transmit::send_telemetry_data(
            big_json_snip.to_string(),
            &telemetry_corrected_snippets_dest,
            &api_key
        ).await;
        if resp_maybe.is_err() {
            error!("snippet send failed: {}", resp_maybe.err().unwrap());
            error!("too bad snippet is lost now");
            continue;
        }
    }
}


pub async fn tele_snip_background_task(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        send_finished_snippets(global_context.clone()).await;
    }
}
