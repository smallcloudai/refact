use std::sync::Arc;
use parking_lot::Mutex as ParkMutex;
use tokio::sync::RwLock as ARwLock;
use rusqlite::params;
use serde_json::json;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::Deserialize;
use async_stream::stream;

use crate::memdb::db_structs::{MemDB, Chore, ChoreEvent};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Deserialize, Default)]
pub struct ChoresSubscriptionPost {
    pub quicksearch: String,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub only_archived: bool,
}


pub fn chore_set(
    mdb: Arc<ParkMutex<MemDB>>,
    chore: Chore,
) -> Result<(), String> {
    fn _chore_set_lowlevel(
        tx: &rusqlite::Transaction,
        chore: &Chore,
    ) -> Result<usize, String> {
        let updated_rows = tx.execute(
        "UPDATE chores SET
                chore_title = ?2,
                chore_spontaneous_work_enable = ?3,
                chore_created_ts = ?4,
                chore_archived_ts = ?5
            WHERE chore_id = ?1",
            params![
            chore.chore_id,
            chore.chore_title,
            chore.chore_spontaneous_work_enable,
            chore.chore_created_ts,
            chore.chore_archived_ts,
        ],
        ).map_err(|e| e.to_string())?;
        if updated_rows == 0 {
            tx.execute(
            "INSERT INTO chores (
                chore_id,
                chore_title,
                chore_spontaneous_work_enable,
                chore_created_ts,
                chore_archived_ts
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                chore.chore_id,
                chore.chore_title,
                chore.chore_spontaneous_work_enable,
                chore.chore_created_ts,
                chore.chore_archived_ts,
            ],
            ).map_err(|e| e.to_string())
        } else {
            Ok(updated_rows)
        }
    }

    // TODO agent: avoid unwraps and expect
    let lite = mdb.lock().lite.clone();
    let mut conn = lite.lock();
    let tx = conn.transaction().expect("Failed to start transaction");
    _chore_set_lowlevel(&tx, &chore).map_err(|e| format!("Failed to insert or replace chore event:\n{e}"))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction:\n{e}"))?;
    Ok(())
}

pub fn chore_event_set(
    mdb: Arc<ParkMutex<MemDB>>,
    cevent: ChoreEvent,
) -> Result<(), String> {
    fn _chore_event_set_lowlevel(
        tx: &rusqlite::Transaction,
        cevent: &ChoreEvent,
    ) -> Result<usize, String> {
        let updated_rows = tx.execute(
        "UPDATE chore_events SET
                chore_event_belongs_to_chore_id = ?2,
                chore_event_summary = ?3,
                chore_event_ts = ?4,
                chore_event_link = ?5,
                chore_event_cthread_id = ?6
            WHERE chore_event_id = ?1",
            params![
            cevent.chore_event_id,
            cevent.chore_event_belongs_to_chore_id,
            cevent.chore_event_summary,
            cevent.chore_event_ts,
            cevent.chore_event_link,
            cevent.chore_event_cthread_id,
        ],
        ).map_err(|e| e.to_string())?;
        if updated_rows == 0 {
            tx.execute(
            "INSERT INTO chore_events (
                chore_event_id,
                chore_event_belongs_to_chore_id,
                chore_event_summary,
                chore_event_ts,
                chore_event_link,
                chore_event_cthread_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                cevent.chore_event_id,
                cevent.chore_event_belongs_to_chore_id,
                cevent.chore_event_summary,
                cevent.chore_event_ts,
                cevent.chore_event_link,
                cevent.chore_event_cthread_id,
            ],
            ).map_err(|e| e.to_string())
        } else {
            Ok(updated_rows)
        }
    }

    // TODO agent: avoid unwraps and expect
    let lite = mdb.lock().lite.clone();
    let mut conn = lite.lock();
    let tx = conn.transaction().expect("Failed to start transaction");
    _chore_event_set_lowlevel(&tx, &cevent).map_err(|e| format!("Failed to insert or replace chore event:\n{e}"))?;
    tx.commit().map_err(|e| format!("Failed to commit transaction:\n{e}"))?;
    Ok(())
}

pub fn chore_get(
    mdb: Arc<ParkMutex<MemDB>>,
    chore_id: String,
) -> Result<Chore, String> {
    fn chores_from_rows(
        mut rows: rusqlite::Rows,
    ) -> Vec<Chore> {
        let mut chores = Vec::new();
        while let Some(row) = rows.next().unwrap_or(None) {
            chores.push(Chore {
                chore_id: row.get("chore_id").unwrap(),
                chore_title: row.get("chore_title").unwrap(),
                chore_spontaneous_work_enable: row.get("chore_spontaneous_work_enable").unwrap(),
                chore_created_ts: row.get("chore_created_ts").unwrap(),
                chore_archived_ts: row.get("chore_archived_ts").unwrap(),
            });
        }
        chores
    }

    // TODO agent: avoid unwraps and expect
    let lite = mdb.lock().lite.clone();
    let conn = lite.lock();
    let mut stmt = conn.prepare("SELECT * FROM chores WHERE chore_id = ?1").unwrap();
    let rows = stmt.query(params![chore_id]).map_err(|e| e.to_string())?;
    let chores = chores_from_rows(rows);
    chores.into_iter().next().ok_or_else(|| format!("No Chore found with id: {}", chore_id))
}

pub fn chore_event_get(
    mdb: Arc<ParkMutex<MemDB>>,
    chore_event_id: String,
) -> Result<ChoreEvent, String> {
    let lite = mdb.lock().lite.clone();
    let conn = lite.lock();
    let mut stmt = conn.prepare("SELECT * FROM chore_events WHERE chore_event_id = ?1").unwrap();
    let mut rows = stmt.query(params![chore_event_id]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().unwrap_or(None) {
        let event = ChoreEvent {
            chore_event_id: row.get("chore_event_id").unwrap(),
            chore_event_belongs_to_chore_id: row.get("chore_event_belongs_to_chore_id").unwrap(),
            chore_event_summary: row.get("chore_event_summary").unwrap(),
            chore_event_ts: row.get("chore_event_ts").unwrap(),
            chore_event_link: row.get("chore_event_link").unwrap(),
            chore_event_cthread_id: row.get("chore_event_cthread_id").unwrap_or(None),
        };
        Ok(event)
    } else {
        Err(format!("No ChoreEvent found with id: {}", chore_event_id))
    }
}

// HTTP handler
pub async fn handle_db_v1_chore_update(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mdb = gcx.read().await.memdb.clone();

    let incoming_json: serde_json::Value = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let chore_id = incoming_json.get("chore_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

    // TODO agent: avoid unwraps and expect
    let chore_rec = match chore_get(mdb.clone().expect("memdb not initialized"), chore_id.clone()) {
        Ok(existing_chore) => existing_chore,
        Err(_) => Chore {
            chore_id,
            ..Default::default()
        },
    };

    let mut chore_json = serde_json::to_value(&chore_rec).unwrap();
    crate::memdb::merge_json(&mut chore_json, &incoming_json);

    let chore_rec: Chore = serde_json::from_value(chore_json).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Deserialization error: {}", e))
    })?;

    if let Err(e) = chore_set(mdb.clone().expect("memdb not initialized"), chore_rec) {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to set chore: {}", e)));
    }

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"status": "success"}).to_string()))
        .unwrap();

    Ok(response)
}

// HTTP handler
pub async fn handle_db_v1_chore_event_update(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mdb = gcx.read().await.memdb.clone();

    let incoming_json: serde_json::Value = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let chore_event_id = incoming_json.get("chore_event_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

    // TODO agent: avoid unwraps and expect
    let chore_event_rec = match chore_event_get(mdb.clone().expect("memdb not initialized"), chore_event_id.clone()) {
        Ok(existing_event) => existing_event,
        Err(_) => ChoreEvent {
            chore_event_id,
            ..Default::default()
        },
    };

    let mut chore_event_json = serde_json::to_value(&chore_event_rec).unwrap();
    crate::memdb::merge_json(&mut chore_event_json, &incoming_json);

    let chore_event_rec: ChoreEvent = serde_json::from_value(chore_event_json).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Deserialization error: {}", e))
    })?;

    if let Err(e) = chore_event_set(mdb.clone().expect("memdb not initialized"), chore_event_rec) {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to set chore event: {}", e)));
    }

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"status": "success"}).to_string()))
        .unwrap();

    Ok(response)
}

fn _chore_get_with_quicksearch(
    mdb: Arc<ParkMutex<MemDB>>,
    chore_id: String,
    post: &ChoresSubscriptionPost,
) -> Result<(Vec<Chore>, Vec<ChoreEvent>), String> {
    fn chore_events_from_rows(
        mut rows: rusqlite::Rows,
    ) -> Vec<ChoreEvent> {
        let mut events = Vec::new();
        while let Some(row) = rows.next().unwrap_or(None) {
            events.push(ChoreEvent {
                chore_event_id: row.get("chore_event_id").unwrap(),
                chore_event_belongs_to_chore_id: row.get("chore_event_belongs_to_chore_id").unwrap(),
                chore_event_summary: row.get("chore_event_summary").unwrap(),
                chore_event_ts: row.get("chore_event_ts").unwrap(),
                chore_event_link: row.get("chore_event_link").unwrap(),
                chore_event_cthread_id: row.get("chore_event_cthread_id").unwrap_or(None),
            });
        }
        events
    }

    let lite = mdb.lock().lite.clone();
    let conn = lite.lock();

    let query = if chore_id.is_empty() {
        "SELECT c.*, e.chore_event_id, e.chore_event_belongs_to_chore_id, e.chore_event_summary, e.chore_event_ts, e.chore_event_link, e.chore_event_cthread_id
         FROM chores c
         LEFT JOIN chore_events e ON c.chore_id = e.chore_event_belongs_to_chore_id
         WHERE c.chore_title LIKE ?1 AND (?2 = 1 AND c.chore_archived_ts IS NOT NULL OR ?2 = 0 AND c.chore_archived_ts IS NULL)
         LIMIT ?3"
    } else {
        "SELECT c.*, e.chore_event_id, e.chore_event_belongs_to_chore_id, e.chore_event_summary, e.chore_event_ts, e.chore_event_link, e.chore_event_cthread_id
         FROM chores c
         LEFT JOIN chore_events e ON c.chore_id = e.chore_event_belongs_to_chore_id
         WHERE c.chore_id = ?1 AND c.chore_title LIKE ?2 AND (?3 = 1 AND c.chore_archived_ts IS NOT NULL OR ?3 = 0 AND c.chore_archived_ts IS NULL)
         LIMIT ?4"
    };

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let mut rows = if chore_id.is_empty() {
        stmt.query(params![
            format!("%{}%", post.quicksearch),
            post.only_archived as i32,
            post.limit as i64
        ]).map_err(|e| e.to_string())?
    } else {
        stmt.query(params![
            chore_id,
            format!("%{}%", post.quicksearch),
            post.only_archived as i32,
            post.limit as i64
        ]).map_err(|e| e.to_string())?
    };

    let mut chores = Vec::new();
    let mut chore_map = std::collections::HashMap::new();
    while let Some(row) = rows.next().unwrap_or(None) {
        let chore_id: String = row.get("chore_id").unwrap();
        if !chore_map.contains_key(&chore_id) {
            let chore = Chore {
                chore_id: chore_id.clone(),
                chore_title: row.get("chore_title").unwrap(),
                chore_spontaneous_work_enable: row.get("chore_spontaneous_work_enable").unwrap(),
                chore_created_ts: row.get("chore_created_ts").unwrap(),
                chore_archived_ts: row.get("chore_archived_ts").unwrap(),
            };
            chores.push(chore);
            chore_map.insert(chore_id.clone(), true);
        }
    }
    let events = chore_events_from_rows(rows);
    Ok((chores, events))
}

// HTTP handler
pub async fn handle_db_v1_chores_sub(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<ChoresSubscriptionPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e)))?;

    let mdb = gcx.read().await.memdb.clone().expect("memdb not initialized");
    let lite = mdb.lock().lite.clone();
    
    let sse = stream! {
        let mut last_id = 0;
        
        // Send initial data
        let (chores, events) = _chore_get_with_quicksearch(mdb.clone(), "".to_string(), &post)
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        for chore in chores {
            let json = serde_json::to_string(&chore).unwrap();
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", json));
        }
        
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", json));
        }
        
        // Poll for updates
        loop {
            if !crate::memdb::db_pubsub::pubsub_trigerred(gcx.clone(), &mdb, 10).await {
                break;
            }
            
            let (deleted_ids, updated_ids) = _chore_subscription_poll(lite.clone(), &mut last_id)
                .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
            for id in deleted_ids {
                yield Ok::<_, ScratchError>(format!("data: {}\n\n", json!({"deleted": id}).to_string()));
            }
            
            for id in updated_ids {
                let (chores, events) = _chore_get_with_quicksearch(mdb.clone(), id, &post)
                    .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                    
                for chore in chores {
                    let json = serde_json::to_string(&chore).unwrap();
                    yield Ok::<_, ScratchError>(format!("data: {}\n\n", json));
                }
                
                for event in events {
                    let json = serde_json::to_string(&event).unwrap();
                    yield Ok::<_, ScratchError>(format!("data: {}\n\n", json));
                }
            }
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

fn _chore_subscription_poll(
    lite_arc: Arc<ParkMutex<rusqlite::Connection>>,
    seen_id: &mut i64
) -> Result<(Vec<String>, Vec<String>), String> {
    let conn = lite_arc.lock();
    let mut stmt = conn.prepare("
        SELECT pubevent_id, pubevent_action, pubevent_json
        FROM pubsub_events
        WHERE pubevent_id > ?1
        AND pubevent_channel = 'chores' AND (pubevent_action = 'update' OR pubevent_action = 'delete')
        ORDER BY pubevent_id ASC
    ").unwrap();
    let mut rows = stmt.query([*seen_id]).map_err(|e| format!("Failed to execute query: {}", e))?;
    let mut deleted_chore_ids = Vec::new();
    let mut updated_chore_ids = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("Failed to fetch row: {}", e))? {
        let id: i64 = row.get(0).unwrap();
        let action: String = row.get(1).unwrap();
        let json: String = row.get(2).unwrap();
        let chore_id = match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(parsed_json) => match parsed_json["chore_id"].as_str() {
                Some(id) => id.to_string(),
                None => {
                    tracing::error!("Missing chore_id in JSON: {}", json);
                    *seen_id = id;
                    continue;
                }
            },
            Err(e) => {
                tracing::error!("Failed to parse JSON: {}. Error: {}", json, e);
                *seen_id = id;
                continue;
            }
        };
        match action.as_str() {
            "delete" => deleted_chore_ids.push(chore_id),
            "update" => updated_chore_ids.push(chore_id),
            _ => return Err(format!("Unknown action: {}", action)),
        }
        *seen_id = id;
    }
    Ok((deleted_chore_ids, updated_chore_ids))
}
