use std::sync::Arc;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hashbrown::HashMap;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, DiffChunk};
use crate::custom_error::ScratchError;
use crate::diffs::{ApplyDiffResult, correct_and_validate_chunks, read_files_n_apply_diff_chunks, unwrap_diff_apply_outputs, ApplyDiffOutput, ApplyDiffUnwrapped};
use crate::global_context::GlobalContext;
use crate::http::routers::v1::chat::deserialize_messages_from_post;
use crate::tools::tool_apply_ticket_aux::tickets_parsing::{correct_and_validate_active_ticket, validate_and_correct_ticket, get_tickets_from_messages, TicketToApply};
use crate::tools::tool_apply_ticket::process_ticket;
use crate::tools::tool_apply_ticket_aux::diff_apply::diff_apply;
use crate::tools::tool_apply_ticket_aux::postprocessing_utils::fill_out_already_applied_status;
use crate::tools::tools_execute::unwrap_subchat_params;


#[derive(Deserialize)]
pub struct PatchPost {
    pub messages: Vec<serde_json::Value>,
    pub ticket_id: String,
}

#[derive(Deserialize)]
pub struct PatchApplyAllPost {
    pub messages: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct PatchResponse {
    state: Vec<ApplyDiffUnwrapped>,
    results: Vec<ApplyDiffResult>,
    chunks: Vec<DiffChunk>,
}

#[derive(Serialize)]
pub struct PatchApplyAllResponse {
    chunks: Vec<DiffChunk>,
}

pub fn resolve_diff_apply_outputs(
    outputs: HashMap<usize, ApplyDiffOutput>,
    diff_chunks: &Vec<DiffChunk>,
) -> Result<Vec<ApplyDiffUnwrapped>, String> {
    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, diff_chunks.clone());
    let mut error_message = String::new();
    for output in outputs_unwrapped.iter() {
        if !output.applied {
            let message = if let Some(detail) = &output.detail {
                format!("Cannot apply some of the diff chunk: {}\n", &detail)
            } else {
                "Cannot apply some of the diff chunk".to_string()
            };
            error_message.push_str(&message);
        }
    }
    if !error_message.is_empty() {
        Err(error_message)
    } else {
        Ok(outputs_unwrapped)
    }
}

pub async fn handle_v1_apply_selected_ticket(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PatchPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let messages = deserialize_messages_from_post(&post.messages)?;
    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        global_context.clone(),
        8096,
        10,
        false,
        messages.clone(),
        "".to_string(),
        false,
    ).await));
    let params = unwrap_subchat_params(ccx.clone(), "apply_ticket").await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Failed to unwrap subchat params: {}", e))
    })?;
    {
        let mut ccx_lock = ccx.lock().await;
        ccx_lock.n_ctx = params.subchat_n_ctx;
    }

    let all_tickets = get_tickets_from_messages(global_context.clone(), &messages, None).await;
    let mut ticket = validate_and_correct_ticket(
        global_context.clone(), post.ticket_id, &all_tickets
    ).await.map_err(|(e, _)| { ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e) })?;

    let mut usage = ChatUsage { ..Default::default() };
    let mut diff_chunks = process_ticket(
        ccx.clone(),
        &mut ticket,
        &params,
        &"apply_ticket".to_string(),
        &mut usage,
    ).await.map_err(|(e, _)| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    correct_and_validate_chunks(global_context.clone(), &mut diff_chunks).await
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let (mut results, outputs) = read_files_n_apply_diff_chunks(
        global_context.clone(),
        &diff_chunks,
        &vec![false; diff_chunks.len()],
        &vec![true; diff_chunks.len()],
        10,
    ).await;
    let apply_outputs = resolve_diff_apply_outputs(outputs, &diff_chunks).map_err(|e| {
        ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Failed to unwrap subchat params: {}", e))
    })?;
    fill_out_already_applied_status(ccx.lock().await.global_context.clone(), &mut results).await;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&PatchResponse { 
            results,
            state: apply_outputs, 
            chunks: diff_chunks 
        }).unwrap()))
        .unwrap())
}

pub async fn handle_v1_apply_all_tickets(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PatchApplyAllPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let messages = deserialize_messages_from_post(&post.messages)?;

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        global_context.clone(),
        8096,
        10,
        false,
        messages.clone(),
        "".to_string(),
        false,
    ).await));
    let params = unwrap_subchat_params(ccx.clone(), "apply_ticket").await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Failed to unwrap subchat params: {}", e))
    })?;
    {
        let mut ccx_lock = ccx.lock().await;
        ccx_lock.n_ctx = params.subchat_n_ctx;
    }

    // leave only the latest ticket for each file
    let all_tickets = get_tickets_from_messages(global_context.clone(), &messages, None).await;
    let mut filename_by_ticket: HashMap<String, TicketToApply> = HashMap::new();
    for ticket in all_tickets.values() {
        if let Some(el) = filename_by_ticket.get(&ticket.filename) {
            if ticket.message_idx <= el.message_idx {
                continue
            } else {
                filename_by_ticket.remove(&ticket.filename);
            }
        }
        let mut ticket = ticket.clone();
        correct_and_validate_active_ticket(global_context.clone(), &mut ticket).await.map_err(|e|
            ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Invalid ticket: {e}"))
        )?;
        filename_by_ticket.insert(ticket.filename.clone(), ticket);
    }

    let mut usage = ChatUsage { ..Default::default() };
    let mut all_diff_chunks = vec![];
    for mut ticket in filename_by_ticket.into_values() {
        let diff_chunks_maybe = process_ticket(
            ccx.clone(),
            &mut ticket,
            &params,
            &"apply_ticket".to_string(),
            &mut usage,
        ).await;
        let mut diff_chunks = diff_chunks_maybe.map_err(|(e, _)|
            ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e)
        )?;
        diff_apply(global_context.clone(), &mut diff_chunks).await.map_err(|err| ScratchError::new(
            StatusCode::UNPROCESSABLE_ENTITY, format!("Couldn't apply the diff: {err}"))
        )?;
        all_diff_chunks.extend(diff_chunks);
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&PatchApplyAllResponse {
            chunks: all_diff_chunks
        }).unwrap()))
        .unwrap())
}
