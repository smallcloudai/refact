use std::collections::HashSet;
use std::sync::Arc;
use async_stream::stream;
use tokio::sync::RwLock as ARwLock;
use serde_json::json;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde::Deserialize;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::knowledge::MemdbSubEvent;

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

    let vec_db = gcx.read().await.vec_db.clone();
    let memid = crate::vecdb::vdb_highlev::memories_add(
        vec_db,
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

    let vec_db = gcx.read().await.vec_db.clone();
    let erased_cnt = crate::vecdb::vdb_highlev::memories_erase(vec_db, &post.memid).await.map_err(|e| {
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

    let vec_db = gcx.read().await.vec_db.clone();
    let upd_cnt = crate::vecdb::vdb_highlev::memories_update(
        vec_db, &post.memid, &post.mem_type, &post.goal, &post.project, &post.payload, &post.origin,
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

    let vec_db = gcx.read().await.vec_db.clone();
    let updated_cnt = crate::vecdb::vdb_highlev::memories_update_used(
        vec_db,
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
    let vec_db = gcx.read().await.vec_db.clone();
    crate::vecdb::vdb_highlev::memories_block_until_vectorized(vec_db, 20_000)
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
    fn _get_last_memid(events: &Vec<MemdbSubEvent>) -> i64 {
        events
            .iter()
            .max_by_key(|x| x.pubevent_id)
            .map(|x| x.pubevent_id)
            .unwrap_or(0)
    }
    let post: MemSubscriptionPost = serde_json::from_slice(&body_bytes).unwrap_or(MemSubscriptionPost::default());

    let vec_db = gcx.read().await.vec_db.clone();
    let mut last_pubevent_id = _get_last_memid(
        &crate::vecdb::vdb_highlev::memdb_subscription_poll(vec_db.clone(), None).await
            .map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?
    );

    let (preexisting_memories, maybe_memids_to_keep) = if let Some(quick_search_query) = post.quick_search {
        let mut preexisting_memories = crate::vecdb::vdb_highlev::memories_select_like(vec_db.clone(), &quick_search_query).await.
            map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?;
        if let Some(limit) = post.limit {
            preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
        }
        let memids_to_keep = preexisting_memories.iter().map(|x| x.memid.clone()).collect::<HashSet<String>>();
        (preexisting_memories, Some(memids_to_keep))
    } else {
        let mut preexisting_memories = crate::vecdb::vdb_highlev::memories_select_all(vec_db.clone()).await.
            map_err(|e| {
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
            })?;
        if let Some(limit) = post.limit {
            preexisting_memories = preexisting_memories.into_iter().take(limit).collect();
        }
        (preexisting_memories, None)
    };

    let sse = stream! {
        for memory in preexisting_memories.iter() {
            if let Some(memids_to_keep) = &maybe_memids_to_keep {
                if !memids_to_keep.contains(&memory.memid) {
                    continue;
                }
            }
            let e = json!({
                "pubevent_id": -1,
                "pubevent_action": "INSERT",
                "pubevent_memid": memory.memid,
                "pubevent_json": serde_json::to_string(&memory).unwrap(),
            });
            yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&e).unwrap()));
        }
        
        loop {
            match crate::vecdb::vdb_highlev::memdb_pubsub_trigerred(vec_db.clone(), 10).await {
                Ok(_) => {}
                Err(_) => {
                    break;
                }
            };
            match crate::vecdb::vdb_highlev::memdb_subscription_poll(vec_db.clone(), Some(last_pubevent_id)).await {
                Ok(new_events) => {
                    for event in new_events.iter() {
                        if let Some(memids_to_keep) = &maybe_memids_to_keep {
                            if !memids_to_keep.contains(&event.pubevent_memid) {
                                continue;
                            }
                        }
                        yield Ok::<_, ScratchError>(format!("data: {}\n\n", serde_json::to_string(&event).unwrap()));
                    }
                    if !new_events.is_empty() {
                        last_pubevent_id = _get_last_memid(&new_events);
                    }
                },
                Err(e) => {
                    tracing::error!(e);
                    break;
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
