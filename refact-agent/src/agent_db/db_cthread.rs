use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use parking_lot::Mutex as ParkMutex;
use serde_json::json;
use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::Deserialize;
use async_stream::stream;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::agent_db::db_structs::{ChoreDB, CThread};
use crate::agent_db::chore_pubsub_sleeping_procedure;


pub fn cthread_get(
    cdb: Arc<ParkMutex<ChoreDB>>,
    cthread_id: String,
) -> Result<CThread, String> {
    let lite = cdb.lock().lite.clone();
    let conn = lite.lock();
    let mut stmt = conn.prepare("SELECT * FROM cthreads WHERE cthread_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query(rusqlite::params![cthread_id])
        .map_err(|e| e.to_string())?;
    let mut cthreads = cthreads_from_rows(rows);
    cthreads.pop().ok_or_else(|| format!("No CThread found with id: {}", cthread_id))
}

pub fn cthreads_from_rows(
    mut rows: rusqlite::Rows,
) -> Vec<CThread> {
    let mut cthreads = Vec::new();
    while let Some(row) = rows.next().unwrap_or(None) {
        cthreads.push(CThread {
            cthread_id: row.get("cthread_id").unwrap(),
            cthread_belongs_to_chore_event_id: row.get::<_, Option<String>>("cthread_belongs_to_chore_event_id").unwrap(),
            cthread_title: row.get("cthread_title").unwrap(),
            cthread_toolset: row.get("cthread_toolset").unwrap(),
            cthread_model: row.get("cthread_model").unwrap(),
            cthread_temperature: row.get("cthread_temperature").unwrap(),
            cthread_max_new_tokens: row.get("cthread_max_new_tokens").unwrap(),
            cthread_n: row.get("cthread_n").unwrap(),
            cthread_error: row.get("cthread_error").unwrap(),
            cthread_anything_new: row.get("cthread_anything_new").unwrap(),
            cthread_created_ts: row.get("cthread_created_ts").unwrap(),
            cthread_updated_ts: row.get("cthread_updated_ts").unwrap(),
            cthread_archived_ts: row.get("cthread_archived_ts").unwrap(),
            cthread_locked_by: row.get("cthread_locked_by").unwrap(),
            cthread_locked_ts: row.get("cthread_locked_ts").unwrap(),
            ..Default::default()
        });
    }
    cthreads
}

pub fn cthread_set_lowlevel(
    tx: &rusqlite::Transaction,
    cthread: &CThread,
) -> Result<usize, String> {
    let updated_rows = tx.execute(
        "UPDATE cthreads SET
            cthread_belongs_to_chore_event_id = ?2,
            cthread_title = ?3,
            cthread_toolset = ?4,
            cthread_model = ?5,
            cthread_temperature = ?6,
            cthread_max_new_tokens = ?7,
            cthread_n = ?8,
            cthread_error = ?9,
            cthread_anything_new = ?10,
            cthread_created_ts = ?11,
            cthread_updated_ts = ?12,
            cthread_archived_ts = ?13,
            cthread_locked_by = ?14,
            cthread_locked_ts = ?15
        WHERE cthread_id = ?1",
        rusqlite::params![
            cthread.cthread_id,
            cthread.cthread_belongs_to_chore_event_id,
            cthread.cthread_title,
            cthread.cthread_toolset,
            cthread.cthread_model,
            cthread.cthread_temperature,
            cthread.cthread_max_new_tokens,
            cthread.cthread_n,
            cthread.cthread_error,
            cthread.cthread_anything_new,
            cthread.cthread_created_ts,
            cthread.cthread_updated_ts,
            cthread.cthread_archived_ts,
            cthread.cthread_locked_by,
            cthread.cthread_locked_ts,
        ],
    ).map_err(|e| e.to_string())?;
    if updated_rows == 0 {
        tx.execute(
            "INSERT INTO cthreads (
                cthread_id,
                cthread_belongs_to_chore_event_id,
                cthread_title,
                cthread_toolset,
                cthread_model,
                cthread_temperature,
                cthread_max_new_tokens,
                cthread_n,
                cthread_error,
                cthread_anything_new,
                cthread_created_ts,
                cthread_updated_ts,
                cthread_archived_ts,
                cthread_locked_by,
                cthread_locked_ts
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                cthread.cthread_id,
                cthread.cthread_belongs_to_chore_event_id,
                cthread.cthread_title,
                cthread.cthread_toolset,
                cthread.cthread_model,
                cthread.cthread_temperature,
                cthread.cthread_max_new_tokens,
                cthread.cthread_n,
                cthread.cthread_error,
                cthread.cthread_anything_new,
                cthread.cthread_created_ts,
                cthread.cthread_updated_ts,
                cthread.cthread_archived_ts,
                cthread.cthread_locked_by,
                cthread.cthread_locked_ts,
            ],
        ).map_err(|e| e.to_string())
    } else {
        Ok(updated_rows)
    }
}

pub fn cthread_set(
    cdb: Arc<ParkMutex<ChoreDB>>,
    cthread: &CThread,
) {
    let (lite, chore_sleeping_point) = {
        let db = cdb.lock();
        (db.lite.clone(), db.chore_sleeping_point.clone())
    };
    {
        let mut conn = lite.lock();
        let tx = conn.transaction().expect("Failed to start transaction");
        if let Err(e) = cthread_set_lowlevel(&tx, cthread) {
            tracing::error!("Failed to insert or replace cthread:\n{}", e);
        }
        let j = serde_json::json!({
            "cthread_id": cthread.cthread_id,
            "cthread_belongs_to_chore_event_id": cthread.cthread_belongs_to_chore_event_id,
        });
        crate::agent_db::chore_pubub_push(&tx, "cthread", "update", &j);
        if let Err(e) = tx.commit() {
            tracing::error!("Failed to commit transaction:\n{}", e);
            return;
        }
    }
    chore_sleeping_point.notify_waiters();
}

pub fn cthread_apply_json(
    cdb: Arc<ParkMutex<ChoreDB>>,
    incoming_json: serde_json::Value,
) -> Result<CThread, String> {
    let cthread_id = incoming_json.get("cthread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "cthread_id is required".to_string())?
        .to_string();
    // all default values if not found, as a way to create new cthreads
    let mut cthread_rec = cthread_get(cdb.clone(), cthread_id.clone()).unwrap_or_default();
    let mut chat_thread_json = serde_json::to_value(&cthread_rec).unwrap();
    crate::agent_db::merge_json(&mut chat_thread_json, &incoming_json);
    cthread_rec = serde_json::from_value(chat_thread_json).unwrap();
    cthread_set(cdb, &cthread_rec);
    Ok(cthread_rec)
}

// HTTP handler
pub async fn handle_db_v1_cthread_update(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let cdb = gcx.read().await.chore_db.clone();

    let incoming_json: serde_json::Value = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON parsing error: {}", e))
    })?;

    let cthread_rec = cthread_apply_json(cdb, incoming_json).map_err(|e| {
        tracing::error!("Failed to apply JSON: {}", e);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Failed to apply JSON: {}", e))
    })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"status": "success", "cthread": cthread_rec}).to_string()))
        .unwrap();

    Ok(response)
}

#[derive(Deserialize)]
pub struct CThreadSubscription {
    #[serde(default)]
    pub quicksearch: String,
    #[serde(default)]
    pub limit: usize,
}

// HTTP handler
pub async fn handle_db_v1_cthreads_sub(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mut post: CThreadSubscription = serde_json::from_slice(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    if post.limit == 0 {
        post.limit = 100;
    }

    let cdb = gcx.read().await.chore_db.clone();
    let lite_arc = cdb.lock().lite.clone();

    let (pre_existing_cthreads, mut last_pubsub_id) = {
        let lite = cdb.lock().lite.clone();
        let max_event_id: i64 = lite.lock().query_row("SELECT COALESCE(MAX(pubevent_id), 0) FROM pubsub_events", [], |row| row.get(0))
            .map_err(|e| { ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get max event ID: {}", e)) })?;
        let cthreads = cthread_quicksearch(cdb.clone(), &String::new(), &post).map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {}", e))
        })?;
        (cthreads, max_event_id)
    };

    let sse = stream! {
        for cthread in pre_existing_cthreads {
            let e = json!({
                "sub_event": "cthread_update",
                "cthread_rec": cthread
            });
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&e).unwrap()));
        }
        loop {
            if !chore_pubsub_sleeping_procedure(gcx.clone(), &cdb, 10).await {
                break;
            }
            let (deleted_cthread_ids, updated_cthread_ids) = match cthread_subsription_poll(lite_arc.clone(), &mut last_pubsub_id) {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!("handle_db_v1_cthreads_sub(1): {}", e);
                    break;
                }
            };

            for deleted_id in deleted_cthread_ids {
                let delete_event = json!({
                    "sub_event": "cthread_delete",
                    "cthread_id": deleted_id,
                });
                yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&delete_event).unwrap()));
            }
            for updated_id in updated_cthread_ids {
                // XXX idea: remember cthread_ids sent to client to filter, instead of quicksearch again here
                match cthread_quicksearch(cdb.clone(), &updated_id, &post) {
                    Ok(updated_cthreads) => {
                        for updated_cthread in updated_cthreads {
                            let update_event = json!({
                                "sub_event": "cthread_update",
                                "cthread_rec": updated_cthread
                            });
                            yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&update_event).unwrap()));
                        }
                    },
                    Err(e) => {
                        tracing::error!("handle_db_v1_cthreads_sub(2): {}", e);
                        continue;
                    }
                }
            }
        }
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .body(Body::wrap_stream(sse))
        .unwrap();

    Ok(response)
}

pub fn cthread_quicksearch(
    cdb: Arc<ParkMutex<ChoreDB>>,
    cthread_id: &String,
    post: &CThreadSubscription,
) -> Result<Vec<CThread>, String> {
    let lite = cdb.lock().lite.clone();
    let conn = lite.lock();
    let query = if cthread_id.is_empty() {
        "SELECT * FROM cthreads WHERE cthread_title LIKE ?1 ORDER BY cthread_id LIMIT ?2"
    } else {
        "SELECT * FROM cthreads WHERE cthread_id = ?1 AND cthread_title LIKE ?2 ORDER BY cthread_id LIMIT ?3"
    };
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let rows = if cthread_id.is_empty() {
        stmt.query(rusqlite::params![format!("%{}%", post.quicksearch), post.limit])
    } else {
        stmt.query(rusqlite::params![cthread_id, format!("%{}%", post.quicksearch), post.limit])
    }.map_err(|e| e.to_string())?;
    Ok(cthreads_from_rows(rows))
}

pub fn cthread_subsription_poll(
    lite_arc: Arc<ParkMutex<rusqlite::Connection>>,
    seen_id: &mut i64
) -> Result<(Vec<String>, Vec<String>), String> {
    let conn = lite_arc.lock();
    let mut stmt = conn.prepare("
        SELECT pubevent_id, pubevent_action, pubevent_json
        FROM pubsub_events
        WHERE pubevent_id > ?1
        AND pubevent_channel = 'cthread' AND (pubevent_action = 'update' OR pubevent_action = 'delete')
        ORDER BY pubevent_id ASC
    ").unwrap();
    let mut rows = stmt.query([*seen_id]).map_err(|e| format!("Failed to execute query: {}", e))?;
    let mut deleted_cthread_ids = Vec::new();
    let mut updated_cthread_ids = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("Failed to fetch row: {}", e))? {
        let id: i64 = row.get(0).unwrap();
        let action: String = row.get(1).unwrap();
        let json: String = row.get(2).unwrap();
        let cthread_id = match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(parsed_json) => match parsed_json["cthread_id"].as_str() {
                Some(id) => id.to_string(),
                None => {
                    tracing::error!("Missing cthread_id in JSON: {}", json);
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
            "delete" => deleted_cthread_ids.push(cthread_id),
            "update" => updated_cthread_ids.push(cthread_id),
            _ => return Err(format!("Unknown action: {}", action)),
        }
        *seen_id = id;
    }
    Ok((deleted_cthread_ids, updated_cthread_ids))
}
