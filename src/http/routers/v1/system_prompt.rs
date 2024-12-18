use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, ChatMeta};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::scratchpads::chat_utils_prompts::prepend_the_right_system_prompt_and_maybe_more_initial_messages;
use crate::scratchpads::scratchpad_utils::HasRagResults;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrependSystemPromptPost {
    pub messages: Vec<ChatMessage>,
    pub chat_meta: ChatMeta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrependSystemPromptResponse {
    pub messages: Vec<ChatMessage>,
    pub messages_to_stream_back: Vec<serde_json::Value>,
}

pub async fn handle_v1_prepend_system_prompt_and_maybe_more_initial_messages(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<PrependSystemPromptPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let mut has_rag_results = HasRagResults::new();

    let messages = prepend_the_right_system_prompt_and_maybe_more_initial_messages(
        gcx.clone(), post.messages, &post.chat_meta, &mut has_rag_results).await;
    let messages_to_stream_back = has_rag_results.in_json;

    Ok(Response::builder()
      .status(StatusCode::OK)
      .body(Body::from(serde_json::to_string(&PrependSystemPromptResponse { messages, messages_to_stream_back }).unwrap()))
      .unwrap())
}
