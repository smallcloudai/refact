use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::scratchpads::chat_utils_prompts::{get_default_system_prompt, system_prompt_add_workspace_info};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemPromptPost {
    #[serde(default)]
    pub have_exploration_tools: bool,
    #[serde(default)]
    pub have_agentic_tools: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemPromptResponse {
    pub system_prompt: String,
}

pub async fn handle_v1_system_prompt(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SystemPromptPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let prompt = get_default_system_prompt(gcx.clone(), post.have_exploration_tools, post.have_agentic_tools).await;

    let prompt_with_workspace_info = system_prompt_add_workspace_info(gcx.clone(), &prompt).await;

    let result = SystemPromptResponse { system_prompt: prompt_with_workspace_info };

    Ok(Response::builder()
      .status(StatusCode::OK)
      .body(Body::from(serde_json::to_string(&result).unwrap()))
      .unwrap())
}