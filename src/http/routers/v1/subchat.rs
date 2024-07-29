use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use crate::at_tools::subchat::execute_subchat;
use crate::call_validation::ChatMessage;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Deserialize)]
struct SubChatPost {
    model_name: String,
    messages: Vec<ChatMessage>,
    wrap_up_depth: usize,
    wrap_up_tokens_cnt: usize,
    tools_turn_on: Vec<String>,
    wrap_up_prompt: String,
}

pub async fn handle_v1_subchat(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SubChatPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let logfn = chrono::Local::now().format("subchat-handler-%Y%m%d-%H%M%S.log").to_string();

    let new_messages = execute_subchat(
        global_context.clone(),
        post.model_name.as_str(),
        post.messages,
        post.tools_turn_on,
        post.wrap_up_depth,
        post.wrap_up_tokens_cnt,
        post.wrap_up_prompt.as_str(),
        logfn,
    ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)))?;

    let resp_serialised = serde_json::to_string_pretty(&new_messages).unwrap();
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(resp_serialised))
            .unwrap()
    )
}
