use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use tracing::info;

use crate::call_validation::ChatPost;
use crate::caps;
use crate::caps::CodeAssistantCaps;
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::scratchpads;

async fn _lookup_chat_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    chat_post: &ChatPost,
) -> Result<(String, String, serde_json::Value), String> {
    let caps_locked = caps.read().unwrap();
    let (model_name, recommended_model_record) =
        caps::which_model_to_use(
            &caps_locked.code_chat_models,
            &chat_post.model,
            &caps_locked.code_chat_default_model,
        )?;
    let (sname, patch) = caps::which_scratchpad_to_use(
        &recommended_model_record.supports_scratchpads,
        &chat_post.scratchpad,
        &recommended_model_record.default_scratchpad,
    )?;
    Ok((model_name, sname.clone(), patch.clone()))
}

pub async fn handle_v1_chat(
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mut chat_post = serde_json::from_slice::<ChatPost>(&body_bytes).map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await?;
    let (model_name, scratchpad_name, scratchpad_patch) = _lookup_chat_scratchpad(
        caps.clone(),
        &chat_post,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e))
    })?;
    if chat_post.parameters.max_new_tokens == 0 {
        chat_post.parameters.max_new_tokens = 2048;
    }
    chat_post.parameters.temperature = Some(chat_post.parameters.temperature.unwrap_or(0.2));
    chat_post.model = model_name.clone();
    let (client1, api_key) = {
        let cx_locked = global_context.write().await;
        (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone())
    };
    let vecdb_search = global_context.read().await.vec_db.clone();
    let mut scratchpad = scratchpads::create_chat_scratchpad(
        global_context.clone(),
        caps,
        model_name.clone(),
        chat_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        vecdb_search,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    let t1 = std::time::Instant::now();
    let prompt = scratchpad.prompt(
        2048,
        &mut chat_post.parameters,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;
    // info!("chat prompt {:?}\n{}", t1.elapsed(), prompt);
    info!("chat prompt {:?}", t1.elapsed());
    crate::restream::scratchpad_interaction_stream(
        global_context.clone(),
        scratchpad,
        "chat-stream".to_string(),
        prompt,
        model_name,
        client1,
        api_key,
        chat_post.parameters.clone(),
    ).await
}