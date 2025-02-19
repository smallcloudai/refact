use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use crate::subchat::{subchat, subchat_single};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::http::routers::v1::chat::deserialize_messages_from_post;


#[derive(Deserialize)]
struct SubChatPost {
    model_name: String,
    messages: Vec<serde_json::Value>,
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
    let messages = deserialize_messages_from_post(&post.messages)?;

    let top_n = 7;
    let fake_n_ctx = 4096;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(
        AtCommandsContext::new(global_context.clone(), fake_n_ctx, top_n, false, messages.clone(), "".to_string(), false).await
    ));

    let new_messages = subchat(
        ccx.clone(),
        post.model_name.as_str(),
        messages,
        post.tools_turn_on,
        post.wrap_up_depth,
        post.wrap_up_tokens_cnt,
        post.wrap_up_prompt.as_str(),
        1,
        None,
        None,
        None,
        Some(false),  
    ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)))?;

    let new_messages = new_messages.into_iter()
        .map(|msgs|msgs.iter().map(|msg|msg.into_value(&None)).collect::<Vec<_>>())
       .collect::<Vec<Vec<_>>>();
    let resp_serialised = serde_json::to_string_pretty(&new_messages).unwrap();
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(resp_serialised))
            .unwrap()
    )
}

#[derive(Deserialize)]
struct SubChatSinglePost {
    model_name: String,
    messages: Vec<serde_json::Value>,
    tools_turn_on: Vec<String>,
    tool_choice: Option<String>,
    only_deterministic_messages: bool,
    temperature: Option<f32>,
    #[serde(default = "default_n")]
    n: usize,
}

fn default_n() -> usize { 1 }

pub async fn handle_v1_subchat_single(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SubChatSinglePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let messages = deserialize_messages_from_post(&post.messages)?;

    let top_n = 7;
    let fake_n_ctx = 4096;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(
        AtCommandsContext::new(global_context.clone(), fake_n_ctx, top_n, false, messages.clone(), "".to_string(), false).await)
    );

    let new_messages = subchat_single(
        ccx.clone(),
        post.model_name.as_str(),
        messages,
        Some(post.tools_turn_on),
        post.tool_choice,
        post.only_deterministic_messages,
        post.temperature,
        None,
        post.n,
        None,
        false,
        None,
        None,
        None,
    ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)))?;

    let new_messages = new_messages.into_iter()
        .map(|msgs|msgs.iter().map(|msg|msg.into_value(&None)).collect::<Vec<_>>())
        .collect::<Vec<Vec<_>>>();
    let resp_serialised = serde_json::to_string_pretty(&new_messages).unwrap();
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(resp_serialised))
            .unwrap()
    )
}
