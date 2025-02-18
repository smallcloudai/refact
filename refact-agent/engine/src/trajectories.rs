use crate::global_context::GlobalContext;
use crate::vecdb::vdb_highlev::{memories_add, memories_block_until_vectorized, memories_erase, memories_select_all, VecDb};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use tracing::info;
use chrono::{NaiveDateTime, Utc};

// NOTE: if you're going to use it with local https proxy make sure that you set insecure flag from cmdline
static URL: &str = "https://www.smallcloud.ai/v1/trajectory-get-all";
static TRAJECTORIES_STATUS_FILENAME: &str = "trajectories_last_update";
static TRAJECTORIES_UPDATE_EACH_N_DAYS: i64 = 7;


async fn save_last_download_time(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let now = Utc::now().naive_utc();
    let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let file_path = cache_dir.join(TRAJECTORIES_STATUS_FILENAME);
    tokio::fs::write(file_path, now_str).await.map_err(|x| x.to_string())
}

async fn is_time_to_download_trajectories(gcx: Arc<ARwLock<GlobalContext>>) -> Result<bool, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let file_path = cache_dir.join(TRAJECTORIES_STATUS_FILENAME);
    let last_download_time = match tokio::fs::read_to_string(file_path).await {
        Ok(time_str) => {
            NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|x| x.to_string())?
        }
        Err(_) => {
            return Ok(true);
        }
    };
    let now = Utc::now().naive_utc();
    let duration_since_last_download = now.signed_duration_since(last_download_time);
    Ok(duration_since_last_download.num_days() >= TRAJECTORIES_UPDATE_EACH_N_DAYS)
}

async fn remove_legacy_trajectories(vecdb: Arc<AMutex<Option<VecDb>>>) -> Result<(), String> {
    for memo in memories_select_all(vecdb.clone())
        .await?
        .iter()
        .filter(|x| x.m_origin == "refact-standard") {
        memories_erase(vecdb.clone(), &memo.memid).await?;
        info!("removed legacy trajectory: {}", memo.memid);
    }
    Ok(())
}

pub async fn try_to_download_trajectories(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> {
    if !is_time_to_download_trajectories(gcx.clone()).await? {
        return Ok(());
    }
    
    let (vec_db, api_key) = {
        let gcx_locked = gcx.read().await;
        (
            gcx_locked.vec_db.clone(),
            gcx_locked.cmdline.api_key.clone(),
        )
    };
    if vec_db.lock().await.is_none() {
        return Err("vecdb is not initialized".to_string());        
    }
    memories_block_until_vectorized(vec_db.clone(), 20_000).await?;

    info!("starting to download trajectories...");
    let client = reqwest::Client::new();
    let response = client
        .get(URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let response_json: Value = response.json().await.map_err(|err| err.to_string())?;
    if response_json["retcode"] != "OK" {
        return Err(format!("failed to download trajectories: {:?}", response_json));
    }

    let trajectories = response_json["data"].as_array().unwrap();
    remove_legacy_trajectories(vec_db.clone()).await?;
    for trajectory in trajectories {
        let m_type = trajectory["kind"].as_str().unwrap_or("unknown");
        let m_goal = trajectory["goal"].as_str().unwrap_or("unknown");
        let m_project = trajectory["framework"].as_str().unwrap_or("unknown");
        let m_payload = trajectory["payload"].as_str().unwrap_or("");
        let m_origin = trajectory["origin"].as_str().unwrap_or("refact-standard");
        if m_payload.is_empty() {
            info!("empty or no payload for the trajectory, skipping it");
            continue;            
        }
        match memories_add(
            vec_db.clone(),
            m_type,
            m_goal,
            m_project,
            m_payload,
            m_origin,
        ).await {
            Ok(memid) => info!("memory added with ID: {}", memid),
            Err(err) => info!("failed to add memory: {}", err),
        }
        info!(
            "downloaded trajectory: type={}, goal={}, project={}, payload={}",
            m_type,
            m_goal,
            m_project,
            crate::nicer_logs::first_n_chars(&m_payload.to_string(), 100)
        );
    }

    info!("finished downloading trajectories");
    save_last_download_time(gcx.clone()).await?;
    Ok(())
}
