use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use axum::extract::Path;
use axum::http::{Response, StatusCode};
use axum::Extension;
use hyper::Body;
use tokio::sync::{broadcast, RwLock as ARwLock};

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

use super::types::*;
use super::session::get_or_create_session_with_trajectory;
use super::content::validate_content_with_attachments;
use super::queue::process_command_queue;

pub async fn handle_v1_chat_subscribe(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Response<Body>, ScratchError> {
    let chat_id = params.get("chat_id")
        .ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "chat_id required".to_string()))?
        .clone();

    let sessions = {
        let gcx_locked = gcx.read().await;
        gcx_locked.chat_sessions.clone()
    };

    let session_arc = get_or_create_session_with_trajectory(gcx.clone(), &sessions, &chat_id).await;
    let session = session_arc.lock().await;
    let snapshot = session.snapshot();
    let mut rx = session.subscribe();
    let initial_seq = session.event_seq;
    drop(session);

    let initial_envelope = EventEnvelope {
        chat_id: chat_id.clone(),
        seq: initial_seq,
        event: snapshot,
    };

    let session_for_stream = session_arc.clone();
    let chat_id_for_stream = chat_id.clone();

    let stream = async_stream::stream! {
        let json = serde_json::to_string(&initial_envelope).unwrap_or_default();
        yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", json));

        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let json = serde_json::to_string(&envelope).unwrap_or_default();
                    yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", json));
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::info!("SSE subscriber lagged, skipped {} events, sending fresh snapshot", skipped);
                    let session = session_for_stream.lock().await;
                    let recovery_envelope = EventEnvelope {
                        chat_id: chat_id_for_stream.clone(),
                        seq: session.event_seq,
                        event: session.snapshot(),
                    };
                    drop(session);
                    let json = serde_json::to_string(&recovery_envelope).unwrap_or_default();
                    yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", json));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::wrap_stream(stream))
        .unwrap())
}

pub async fn handle_v1_chat_command(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    Path(chat_id): Path<String>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let request: CommandRequest = serde_json::from_slice(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    let sessions = {
        let gcx_locked = gcx.read().await;
        gcx_locked.chat_sessions.clone()
    };

    let session_arc = get_or_create_session_with_trajectory(gcx.clone(), &sessions, &chat_id).await;
    let mut session = session_arc.lock().await;

    if session.is_duplicate_request(&request.client_request_id) {
        session.emit(ChatEvent::Ack {
            client_request_id: request.client_request_id.clone(),
            accepted: true,
            result: Some(serde_json::json!({"duplicate": true})),
        });
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"status":"duplicate"}"#))
            .unwrap());
    }

    if matches!(request.command, ChatCommand::Abort {}) {
        session.abort_stream();
        session.emit(ChatEvent::Ack {
            client_request_id: request.client_request_id,
            accepted: true,
            result: Some(serde_json::json!({"aborted": true})),
        });
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"status":"aborted"}"#))
            .unwrap());
    }

    if session.command_queue.len() >= MAX_QUEUE_SIZE {
        session.emit(ChatEvent::Ack {
            client_request_id: request.client_request_id,
            accepted: false,
            result: Some(serde_json::json!({"error": "queue full"})),
        });
        return Ok(Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"status":"queue_full"}"#))
            .unwrap());
    }

    let validation_error = match &request.command {
        ChatCommand::UserMessage { content, attachments } => {
            validate_content_with_attachments(content, attachments).err()
        }
        ChatCommand::RetryFromIndex { content, attachments, .. } => {
            validate_content_with_attachments(content, attachments).err()
        }
        ChatCommand::UpdateMessage { content, attachments, .. } => {
            validate_content_with_attachments(content, attachments).err()
        }
        _ => None,
    };

    if let Some(error) = validation_error {
        session.emit(ChatEvent::Ack {
            client_request_id: request.client_request_id,
            accepted: false,
            result: Some(serde_json::json!({"error": error})),
        });
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Body::from(format!(r#"{{"status":"invalid_content","error":"{}"}}"#, error)))
            .unwrap());
    }

    session.command_queue.push_back(request.clone());
    session.runtime.queue_size = session.command_queue.len();
    session.touch();

    session.emit(ChatEvent::Ack {
        client_request_id: request.client_request_id,
        accepted: true,
        result: Some(serde_json::json!({"queued": true})),
    });

    let queue_notify = session.queue_notify.clone();
    let processor_running = session.queue_processor_running.clone();
    drop(session);

    if !processor_running.swap(true, Ordering::SeqCst) {
        tokio::spawn(process_command_queue(gcx, session_arc, processor_running));
    } else {
        queue_notify.notify_one();
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status":"accepted"}"#))
        .unwrap())
}
