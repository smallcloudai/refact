use std::collections::HashSet;
use std::sync::Arc;
use async_stream::stream;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use serde_json::json;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use tracing::warn;
use serde::Deserialize;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::memdb::db_pubsub::PubSubEvent;

#[derive(Deserialize)]
struct MemAddRequest {
    mem_type: String,
    goal: String,
    project: String,
    payload: String,
    origin: String,   // TODO: upgrade to serde_json::Value
}

#[derive(Deserialize)]
struct MemUpdateRequest {
    memid: String,
    mem_type: String,
    goal: String,
    project: String,
    payload: String,
    origin: String,   // TODO: upgrade to serde_json::Value
}

#[derive(Deserialize)]
struct MemEraseRequest {
    memid: String,
}

#[derive(Deserialize)]
struct MemUpdateUsedRequest {
    memid: String,
    correct: i32,
    relevant: i32,
}

pub async fn handle_mem_add(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemAddRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db,
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string())),
    };
    
    let vec_service = match gcx.read().await.vectorizer_service.lock().await.as_ref() {
        Some(service) => Arc::new(AMutex::new(service.clone())),
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "vectorizer service not initialized".to_string())),
    };
    let memid = crate::memdb::db_memories::memories_add(
        memdb,
        vec_service,
        &post.mem_type,
        &post.goal,
        &post.project,
        &post.payload,
        &post.origin,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"memid": memid})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_erase(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemEraseRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db,
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string())),
    };
    let erased_cnt = crate::memdb::db_memories::memories_erase(memdb, &post.memid).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": erased_cnt > 0})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_upd(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemUpdateRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db,
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string())),
    };
    
    let vec_service = match gcx.read().await.vectorizer_service.lock().await.as_ref() {
        Some(service) => Arc::new(AMutex::new(service.clone())),
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "vectorizer service not initialized".to_string())),
    };
    let upd_cnt = crate::memdb::db_memories::memories_update(
        memdb, vec_service, &post.memid, &post.mem_type, &post.goal, &post.project, &post.payload, &post.origin,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": upd_cnt > 0})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_update_used(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post: MemUpdateUsedRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::info!("cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db,
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string())),
    };
    let updated_cnt = crate::memdb::db_memories::memories_update_used(
        memdb,
        &post.memid,
        post.correct,
        post.relevant,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
    })?;

    assert!(updated_cnt <= 1);

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": updated_cnt>0})).unwrap()))
        .unwrap();

    Ok(response)
}

pub async fn handle_mem_block_until_vectorized(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let vec_service = match gcx.read().await.vectorizer_service.lock().await.as_ref() {
        Some(service) => Arc::new(AMutex::new(service.clone())),
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "vectorizer service not initialized".to_string())),
    };
    crate::vecdb::vdb_highlev::memories_block_until_vectorized(vec_service, 20_000)
        .await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&json!({"success": true})).unwrap()))
        .unwrap();

    Ok(response)
}

#[derive(Deserialize, Default)]
pub struct MemSubscriptionPost {
    #[serde(default)]
    pub quick_search: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn handle_mem_sub(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    fn _get_last_memid(events: &Vec<PubSubEvent>) -> i64 {
        events
            .iter()
            .max_by_key(|x| x.pubevent_id)
            .map(|x| x.pubevent_id)
            .unwrap_or(0)
    }
    
    let post: MemSubscriptionPost = serde_json::from_slice(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e)))
        .unwrap_or(MemSubscriptionPost::default());

    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db,
        None => return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "memdb not initialized".to_string())),
    };
    
    let last_pubevent_id = _get_last_memid(
        &crate::memdb::db_pubsub::pubsub_poll(memdb.lock().lite.clone(), &"memories".to_string(), None)
            .map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?
    );

    let (preexisting_memories, maybe_memids_to_keep) = if let Some(quick_search_query) = post.quick_search {
        let mut preexisting_memories = crate::memdb::db_memories::memories_select_like(memdb.clone(), &quick_search_query).await.
            map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?;
        if let Some(limit) = post.limit {
            preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
        }
        let memids_to_keep = preexisting_memories.iter().map(|x| x.memid.clone()).collect::<HashSet<String>>();
        (preexisting_memories, Some(memids_to_keep))
    } else {
        let mut preexisting_memories = crate::memdb::db_memories::memories_select_all(memdb.clone()).await.
            map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?;
        if let Some(limit) = post.limit {
            preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
        }
        (preexisting_memories, None)
    };

    let preexisting_memories_clone = preexisting_memories.clone();
    let maybe_memids_to_keep_clone = maybe_memids_to_keep.clone();
    let memdb_lite = memdb.lock().lite.clone();
    let gcx_clone = gcx.clone();
    let sse = stream! {
        for memory in preexisting_memories_clone.iter() {
            if let Some(memids_to_keep) = &maybe_memids_to_keep_clone {
                if !memids_to_keep.contains(&memory.memid) {
                    continue;
                }
            }
            
            let memory_json = match serde_json::to_string(&memory) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize memory: {}", e);
                    continue;
                }
            };
            
            let e = json!({
                "pubevent_id": -1,
                "pubevent_action": "INSERT",
                "pubevent_memid": memory.memid,
                "pubevent_json": memory_json,
            });
            
            let event_json = match serde_json::to_string(&e) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize event: {}", e);
                    continue;
                }
            };
            
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", event_json));
        }
        
        let mut last_id = last_pubevent_id;
        loop {
            if !crate::memdb::db_pubsub::pubsub_trigerred(gcx.clone(), &memdb, 5).await {
                break;
            }
            match crate::memdb::db_pubsub::pubsub_poll(memdb_lite.clone(), &"memories".to_string(), Some(last_id)) {
                Ok(new_events) => {
                    for event in new_events.iter() {
                        if let Some(memids_to_keep) = &maybe_memids_to_keep_clone {
                            if !memids_to_keep.contains(&event.pubevent_obj_id) {
                                continue;
                            }
                        }
                        let event_json = match serde_json::to_string(&event) {
                            Ok(json) => json,
                            Err(e) => {
                                tracing::error!("Failed to serialize event: {}", e);
                                continue;
                            }
                        };
                        yield Ok::<_, ScratchError>(format!("data: {}\n\n", event_json));
                    }
                    if !new_events.is_empty() {
                        last_id = _get_last_memid(&new_events);
                    }
                },
                Err(e) => {
                    tracing::error!("{}", e);
                    break;
                }
            };
            
            match crate::vecdb::vdb_highlev::get_status(
                gcx_clone.read().await.vec_db.clone(),
                gcx_clone.read().await.vectorizer_service.clone()
            ).await {
                Ok(Some(status)) => {
                    let status_json = match serde_json::to_string(&status) {
                        Ok(json) => json,
                        Err(e) => {
                            tracing::error!("Failed to serialize status: {}", e);
                            continue;
                        }
                    };
                    yield Ok::<_, ScratchError>(format!("data: {}\n\n", status_json));
                },
                Err(err) => {
                    warn!("Error while getting vecdb status: {}", err);
                    continue;
                },
                _ => {
                    warn!("Cannot get vecdb status");
                    continue;
                }
            };
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
