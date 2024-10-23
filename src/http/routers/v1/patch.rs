use std::sync::Arc;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, DiffChunk};
use crate::custom_error::ScratchError;
use crate::diffs::{ApplyDiffResult, ApplyDiffUnwrapped, correct_and_validate_chunks, read_files_n_apply_diff_chunks, unwrap_diff_apply_outputs};
use crate::global_context::GlobalContext;
use crate::http::routers::v1::chat::deserialize_messages_from_post;
use crate::tools::tool_patch_aux::tickets_parsing::{get_and_correct_active_tickets, get_tickets_from_messages};
use crate::tools::tool_patch::process_tickets;
use crate::tools::tools_execute::unwrap_subchat_params;


#[derive(Deserialize)]
pub struct PatchPost {
    pub messages: Vec<serde_json::Value>,
    pub ticket_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct PatchResponse {
    state: Vec<ApplyDiffUnwrapped>,
    results: Vec<ApplyDiffResult>,
    chunks: Vec<DiffChunk>,
    ticket_ids_already_applied: Vec<String>,
}

pub async fn handle_v1_patch_single_file_from_ticket(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PatchPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    if post.ticket_ids.is_empty() {
        return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "'ticket_ids' shouldn't be empty".to_string()));
    }
    let messages = deserialize_messages_from_post(&post.messages)?;

    let ccx = Arc::new(AMutex::new(AtCommandsContext::new(
        global_context.clone(),
        8096,
        10,
        false,
        messages,
        "".to_string(),
    ).await));
    let params = unwrap_subchat_params(ccx.clone(), "patch").await.map_err(|e|{
        ScratchError::new(StatusCode::BAD_REQUEST, format!("Failed to unwrap subchat params: {}", e))
    })?;
    {
        let mut ccx_lock = ccx.lock().await;
        ccx_lock.n_ctx = params.subchat_n_ctx;
    }
    
    let all_tickets_from_above = get_tickets_from_messages(ccx.clone()).await;
    let mut active_tickets = get_and_correct_active_tickets(
        global_context.clone(), post.ticket_ids.clone(), all_tickets_from_above.clone()
    ).await.map_err(|e|{
        ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e)
    })?;

    let mut usage = ChatUsage { ..Default::default() };

    let mut res;
    loop {
        let diff_chunks = process_tickets(
            ccx.clone(),
            &mut active_tickets,
            post.ticket_ids.clone(),
            &params,
            &"patch_123".to_string(),
            &mut usage,
        ).await;
        res = diff_chunks;
        if active_tickets.is_empty() {
            break;
        }
    }
    let mut diff_chunks = res.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    let (
        results, 
        outputs_unwrapped,
        already_applied_tickets,
    ) = if diff_chunks.is_empty() {
        (vec![], vec![], post.ticket_ids.clone())
    } else {
        correct_and_validate_chunks(global_context.clone(), &mut diff_chunks).await
            .map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

        let (results, outputs) = read_files_n_apply_diff_chunks(
            global_context.clone(),
            &diff_chunks,
            &vec![false; diff_chunks.len()],
            &vec![true; diff_chunks.len()],
            10
        ).await;

        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, diff_chunks.clone());
        (results, outputs_unwrapped, vec![])
    };

    let resp = PatchResponse {
        state: outputs_unwrapped,
        results,
        chunks: diff_chunks,
        ticket_ids_already_applied: already_applied_tickets,
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&resp).unwrap()))
        .unwrap())
}
