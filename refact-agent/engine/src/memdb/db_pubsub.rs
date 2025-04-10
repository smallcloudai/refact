use crate::global_context::GlobalContext;
use crate::memdb::db_structs::MemDB;
use parking_lot::Mutex as ParkMutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubEvent {
    pub pubevent_id: i64,
    pub pubevent_channel: String,
    pub pubevent_action: String,
    pub pubevent_obj_id: String,
    pub pubevent_obj_json: String,
    pub pubevent_ts: String,
}

pub async fn pubsub_trigerred(
    gcx: Arc<ARwLock<GlobalContext>>,
    mdb: Arc<ParkMutex<MemDB>>,
    sleep_seconds: u64,
) -> bool {
    let shutdown_flag: Arc<AtomicBool> = gcx.read().await.shutdown_flag.clone();
    if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    let sleeping_point = mdb.lock().memdb_sleeping_point.clone();
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(sleep_seconds),
        sleeping_point.notified(),
    ).await {
        Ok(_) => {}
        Err(_) => {} // timeout
    }
    let should_continue = !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed);
    should_continue
}

pub fn pubsub_poll(
    lite_arc: Arc<ParkMutex<rusqlite::Connection>>,
    pubevent_channel: &String,
    from_pubevent_id: Option<i64>,
) -> rusqlite::Result<Vec<PubSubEvent>, String> {
    let query = "
        SELECT pubevent_id, pubevent_channel, pubevent_action, pubevent_obj_id, pubevent_obj_json, pubevent_ts
        FROM pubsub_events
        WHERE pubevent_channel = ?1 AND pubevent_id > ?2
        ORDER BY pubevent_id ASC
    ";
    let from_id = from_pubevent_id.unwrap_or(0);
    let lite_locked = lite_arc.lock();
    let mut stmt = lite_locked.prepare(query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([pubevent_channel.to_string(), from_id.to_string()], |row| {
            Ok(PubSubEvent {
                pubevent_id: row.get(0)?,
                pubevent_channel: row.get(1)?,
                pubevent_action: row.get(2)?,
                pubevent_obj_id: row.get(3)?,
                pubevent_obj_json: row.get(4)?,
                pubevent_ts: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?)
}


