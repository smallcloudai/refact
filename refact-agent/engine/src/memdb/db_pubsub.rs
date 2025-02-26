use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::memdb::db_structs::MemDB;
use async_stream::stream;

use axum::http::{Response, StatusCode};
use axum::Extension;
use hyper::Body;
use parking_lot::Mutex as ParkMutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
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

#[derive(Deserialize, Default)]
pub struct PubSubSubscriptionPost {
    pub channel: String,
    #[serde(default)]
    pub quick_search: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn pubsub_trigerred(
    gcx: Arc<ARwLock<GlobalContext>>,
    mdb: &Arc<ParkMutex<MemDB>>,
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
    )
    .await
    {
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

pub async fn handle_pubsub(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    fn _get_last_pubevent_id(events: &Vec<PubSubEvent>) -> i64 {
        events
            .iter()
            .max_by_key(|x| x.pubevent_id)
            .map(|x| x.pubevent_id)
            .unwrap_or(0)
    }
    let post = serde_json::from_slice::<PubSubSubscriptionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e)))?;

    let memdb = gcx.read().await.memdb.clone().expect("memdb not initialized");
    let lite = memdb.lock().lite.clone();
    
    let events = pubsub_poll(lite.clone(), &post.channel, None)
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))?;
    let mut last_pubevent_id = _get_last_pubevent_id(&events);

    let (preexisting_items, maybe_obj_ids_to_keep) =
        if let Some(quick_search_query) = post.quick_search {
            let mut preexisting_memories = crate::memdb::db_memories::memories_select_like(
                memdb.clone(),
                &quick_search_query,
            )
            .await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))?;
            if let Some(limit) = post.limit {
                preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
            }
            let memids_to_keep = preexisting_memories
                .iter()
                .map(|x| x.memid.clone())
                .collect::<HashSet<String>>();
            (preexisting_memories, Some(memids_to_keep))
        } else {
            let mut preexisting_memories =
                crate::memdb::db_memories::memories_select_all(memdb.clone())
                    .await
                    .map_err(|e| {
                        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
                    })?;
            if let Some(limit) = post.limit {
                preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
            }
            (preexisting_memories, None)
        };

    let sse = stream! {
        for item in preexisting_items.iter() {
            if let Some(obj_ids_to_keep) = &maybe_obj_ids_to_keep {
                if !obj_ids_to_keep.contains(&item.memid) {
                    continue;
                }
            }
            // pubevent_id, pubevent_channel, pubevent_action, pubevent_obj_id, pubevent_obj_json, pubevent_ts
            let e = json!({
                "pubevent_id": -1,
                "pubevent_channel": post.channel,
                "pubevent_action": "INSERT",
                "pubevent_obj_id": item.memid,
                "pubevent_json": serde_json::to_string(&item).unwrap(),
            });
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&e).unwrap()));
        }

        loop {
            if !pubsub_trigerred(gcx.clone(), &memdb, 5).await {
                break;
            };
            match pubsub_poll(lite.clone(), &post.channel, Some(last_pubevent_id)) {
                Ok(new_events) => {
                    for event in new_events.iter() {
                        if let Some(obj_ids_to_keep) = &maybe_obj_ids_to_keep {
                            if !obj_ids_to_keep.contains(&event.pubevent_obj_id) {
                                continue;
                            }
                        }
                        yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&event).unwrap()));
                    }
                    if !new_events.is_empty() {
                        last_pubevent_id = _get_last_pubevent_id(&new_events);
                    }
                },
                Err(e) => {
                    tracing::error!(e);
                    break;
                }
            };

            // No need to get status anymore
            /*match crate::vecdb::vdb_highlev::get_status(vecdb.clone()).await {
                Ok(Some(status)) => {
                    yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&status).unwrap()));
                },
                Err(err) => {
                    warn!("Error while getting vecdb status: {}", err);
                    continue;
                },
                _ => {
                    warn!("Cannot get vecdb status");
                    continue;
                }
            };*/

        }
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .body(Body::wrap_stream(sse))
        .unwrap();
    Ok(response)
}
