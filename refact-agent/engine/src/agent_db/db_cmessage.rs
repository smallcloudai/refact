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

use crate::agent_db::db_structs::CMessage;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;


pub fn cmessages_from_rows(
    mut rows: rusqlite::Rows,
) -> Vec<CMessage> {
    let mut cmessages = Vec::new();
    while let Some(row) = rows.next().unwrap_or(None) {
        cmessages.push(CMessage {
            cmessage_belongs_to_cthread_id: row.get("cmessage_belongs_to_cthread_id").unwrap(),
            cmessage_alt: row.get("cmessage_alt").unwrap(),
            cmessage_num: row.get("cmessage_num").unwrap(),
            cmessage_prev_alt: row.get("cmessage_prev_alt").unwrap(),
            cmessage_usage_model: row.get("cmessage_usage_model").unwrap(),
            cmessage_usage_prompt: row.get("cmessage_usage_prompt").unwrap(),
            cmessage_usage_completion: row.get("cmessage_usage_completion").unwrap(),
            cmessage_json: row.get("cmessage_json").unwrap(),
        });
    }
    cmessages
}

pub fn cmessage_set_lowlevel(
    tx: &rusqlite::Transaction,
    cmessage: &CMessage,
) -> Result<usize, String> {
    let updated_rows = tx.execute(
        "UPDATE cmessages SET
            cmessage_belongs_to_cthread_id = ?1,
            cmessage_alt = ?2,
            cmessage_num = ?3,
            cmessage_prev_alt = ?4,
            cmessage_usage_model = ?5,
            cmessage_usage_prompt = ?6,
            cmessage_usage_completion = ?7,
            cmessage_json = ?8
        WHERE cmessage_belongs_to_cthread_id = ?1 AND cmessage_alt = ?2 AND cmessage_num = ?3",
        params![
            cmessage.cmessage_belongs_to_cthread_id,
            cmessage.cmessage_alt,
            cmessage.cmessage_num,
            cmessage.cmessage_prev_alt,
            cmessage.cmessage_usage_model,
            cmessage.cmessage_usage_prompt,
            cmessage.cmessage_usage_completion,
            cmessage.cmessage_json,
        ],
    ).map_err(|e| e.to_string())?;
    if updated_rows == 0 {
        tx.execute(
            "INSERT INTO cmessages (
                cmessage_belongs_to_cthread_id,
                cmessage_alt,
                cmessage_num,
                cmessage_prev_alt,
                cmessage_usage_model,
                cmessage_usage_prompt,
                cmessage_usage_completion,
                cmessage_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                cmessage.cmessage_belongs_to_cthread_id,
                cmessage.cmessage_alt,
                cmessage.cmessage_num,
                cmessage.cmessage_prev_alt,
                cmessage.cmessage_usage_model,
                cmessage.cmessage_usage_prompt,
                cmessage.cmessage_usage_completion,
                cmessage.cmessage_json,
            ],
        ).map_err(|e| e.to_string())
    } else {
        Ok(updated_rows)
    }
}

pub fn cmessage_set(
    tx: &rusqlite::Transaction,
    cmessage: CMessage,
) {
    if let Err(e) = cmessage_set_lowlevel(&tx, &cmessage) {
        tracing::error!("Failed to insert or replace cmessage:\n{}", e);
        return;
    }
    let j = serde_json::json!({
        "cmessage_belongs_to_cthread_id": cmessage.cmessage_belongs_to_cthread_id,
        "cmessage_alt": cmessage.cmessage_alt,
        "cmessage_num": cmessage.cmessage_num,
        "cthread_id": cmessage.cmessage_belongs_to_cthread_id,
    });
    crate::agent_db::chore_pubub_push(tx, "cmessage", "update", &j);
    crate::agent_db::chore_pubub_push(tx, "cthread", "update", &j);
}

pub fn cmessage_get(
    tx: &rusqlite::Transaction,
    cmessage_belongs_to_cthread_id: String,
    cmessage_alt: i32,
    cmessage_num: i32,
) -> Result<CMessage, String> {
    let mut stmt = tx.prepare(
        "SELECT * FROM cmessages WHERE cmessage_belongs_to_cthread_id = ?1 AND cmessage_alt = ?2 AND cmessage_num = ?3"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query(params![cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num])
        .map_err(|e| e.to_string())?;
    let cmessages = cmessages_from_rows(rows);
    cmessages.into_iter().next()
        .ok_or_else(|| format!("No CMessage found with {}:{}:{}", cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num))
}

pub fn cmessage_get_with_lite_arc(
    lite_arc: Arc<ParkMutex<rusqlite::Connection>>,
    cmessage_belongs_to_cthread_id: String,
    cmessage_alt: i32,
    cmessage_num: i32,
) -> Result<CMessage, String> {
    let mut conn = lite_arc.lock();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    cmessage_get(&tx, cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num)
}


// HTTP handler
pub async fn handle_db_v1_cmessages_update(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let cdb = gcx.read().await.chore_db.clone();

    let incoming_json: serde_json::Value = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let updates = incoming_json.as_array().ok_or_else(|| {
        ScratchError::new(StatusCode::BAD_REQUEST, "Expected a list of updates".to_string())
    })?;

    let (lite, chore_sleeping_point) = {
        let db = cdb.lock();
        (db.lite.clone(), db.chore_sleeping_point.clone())
    };
    {
        let mut conn = lite.lock();
        let tx = conn.transaction().map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Transaction error: {}", e))
        })?;

        for update in updates {
            let cmessage_belongs_to_cthread_id = update.get("cmessage_belongs_to_cthread_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let cmessage_num = update.get("cmessage_num").and_then(|v| v.as_i64()).unwrap_or_default() as i32;
            let cmessage_alt = update.get("cmessage_alt").and_then(|v| v.as_i64()).unwrap_or_default() as i32;

            let cmessage_rec = match cmessage_get(&tx, cmessage_belongs_to_cthread_id.clone(), cmessage_alt, cmessage_num) {
                Ok(existing_cmessage) => existing_cmessage,
                Err(_) => CMessage {
                    cmessage_belongs_to_cthread_id,
                    cmessage_alt,
                    cmessage_num,
                    ..Default::default()
                },
            };

            let mut cmessage_json = serde_json::to_value(&cmessage_rec).unwrap();
            crate::agent_db::merge_json(&mut cmessage_json, &update);

            let cmessage_rec: CMessage = serde_json::from_value(cmessage_json).map_err(|e| {
                ScratchError::new(StatusCode::BAD_REQUEST, format!("Deserialization error: {}", e))
            })?;

            cmessage_set(&tx, cmessage_rec);
        }
        chore_sleeping_point.notify_waiters();

        tx.commit().map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Commit error: {}", e))
        })?;
    }

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"status": "success"}).to_string()))
        .unwrap();

    Ok(response)
}

#[derive(Deserialize)]
struct CMessagesSubscription {
    cmessage_belongs_to_cthread_id: String,
}

// HTTP handler
pub async fn handle_db_v1_cmessages_sub(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: CMessagesSubscription = serde_json::from_slice(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let cdb = gcx.read().await.chore_db.clone();
    let lite_arc = cdb.lock().lite.clone();

    let (pre_existing_cmessages, mut last_pubsub_id) = {
        let mut conn = lite_arc.lock();
        let tx = conn.transaction().unwrap();

        let mut stmt = tx.prepare("SELECT * FROM cmessages WHERE cmessage_belongs_to_cthread_id = ?1 ORDER BY cmessage_num, cmessage_alt").map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;
        let rows = stmt.query(rusqlite::params![post.cmessage_belongs_to_cthread_id]).map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {}", e))
        })?;
        let cmessages = cmessages_from_rows(rows);

        let max_event_id: i64 = tx.query_row(
            "SELECT COALESCE(MAX(pubevent_id), 0) FROM pubsub_events",
            [],
            |row| row.get(0)
        ).map_err(|e| {
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get max event ID: {}", e))
        })?;

        (cmessages, max_event_id)
    };

    let sse = stream! {
        for cmessage in pre_existing_cmessages {
            let e = json!({
                "sub_event": "cmessage_update",
                "cmessage_rec": cmessage
            });
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&e).unwrap()));
        }

        loop {
            if !crate::agent_db::chore_pubsub_sleeping_procedure(gcx.clone(), &cdb, 10).await {
                break;
            }
            let (deleted_cmessage_keys, updated_cmessage_keys) = match _cmessage_subscription_poll(lite_arc.clone(), &mut last_pubsub_id) {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!("handle_db_v1_cmessages_sub(1): {:?}", e);
                    break;
                }
            };
            for deleted_key in deleted_cmessage_keys {
                if post.cmessage_belongs_to_cthread_id.is_empty() || post.cmessage_belongs_to_cthread_id == deleted_key.cmessage_belongs_to_cthread_id {
                    let delete_event = json!({
                        "sub_event": "cmessage_delete",
                        "cmessage_belongs_to_cthread_id": deleted_key.cmessage_belongs_to_cthread_id,
                        "cmessage_alt": deleted_key.cmessage_alt,
                        "cmessage_num": deleted_key.cmessage_num,
                    });
                    yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&delete_event).unwrap()));
                }
            }
            for updated_key in updated_cmessage_keys {
                let update_event = {
                    match cmessage_get_with_lite_arc(lite_arc.clone(), updated_key.cmessage_belongs_to_cthread_id.clone(), updated_key.cmessage_alt, updated_key.cmessage_num) {
                        Ok(updated_cmessage) => {
                            if post.cmessage_belongs_to_cthread_id.is_empty() || post.cmessage_belongs_to_cthread_id == updated_key.cmessage_belongs_to_cthread_id {
                                json!({
                                    "sub_event": "cmessage_update",
                                    "cmessage_rec": updated_cmessage
                                })
                            } else {
                                continue;
                            }
                        },
                        Err(e) => {
                            tracing::error!("handle_db_v1_cmessages_sub(2): {}", e);
                            break;
                        }
                    }
                };
                yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&update_event).unwrap()));
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

struct _CMessageKey {
    cmessage_belongs_to_cthread_id: String,
    cmessage_alt: i32,
    cmessage_num: i32,
}

fn _cmessage_subscription_poll(
    lite_arc: Arc<ParkMutex<rusqlite::Connection>>,
    seen_id: &mut i64
) -> Result<(Vec<_CMessageKey>, Vec<_CMessageKey>), String> {
    let conn = lite_arc.lock();
    let mut stmt = conn.prepare("
        SELECT pubevent_id, pubevent_action, pubevent_json
        FROM pubsub_events
        WHERE pubevent_id > ?1
        AND pubevent_channel = 'cmessage' AND (pubevent_action = 'update' OR pubevent_action = 'delete')
        ORDER BY pubevent_id ASC
    ").unwrap();
    let mut rows = stmt.query([*seen_id]).map_err(|e| format!("Failed to execute query: {}", e))?;
    let mut deleted_cmessage_keys = Vec::new();
    let mut updated_cmessage_keys = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("Failed to fetch row: {}", e))? {
        let id: i64 = row.get(0).unwrap();
        let action: String = row.get(1).unwrap();
        let json: String = row.get(2).unwrap();
        let cmessage_key = match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(parsed_json) => {
                let cthread_id = parsed_json["cmessage_belongs_to_cthread_id"].as_str();
                let alt = parsed_json["cmessage_alt"].as_i64();
                let num = parsed_json["cmessage_num"].as_i64();
                match (cthread_id, alt, num) {
                    (Some(id), Some(alt), Some(num)) => _CMessageKey {
                        cmessage_belongs_to_cthread_id: id.to_string(),
                        cmessage_alt: alt as i32,
                        cmessage_num: num as i32,
                    },
                    _ => {
                        tracing::error!("Missing or invalid cmessage key components in JSON: {}", json);
                        *seen_id = id;
                        continue;
                    }
                }
            },
            Err(e) => {
                tracing::error!("Failed to parse JSON: {}. Error: {}", json, e);
                *seen_id = id;
                continue;
            }
        };
        match action.as_str() {
            "delete" => deleted_cmessage_keys.push(cmessage_key),
            "update" => updated_cmessage_keys.push(cmessage_key),
            _ => return Err(format!("Unknown action: {}", action)),
        }
        *seen_id = id;
    }
    Ok((deleted_cmessage_keys, updated_cmessage_keys))
}

