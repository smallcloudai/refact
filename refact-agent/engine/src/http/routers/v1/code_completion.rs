use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use tracing::info;
use crate::call_validation::{CodeCompletionPost, code_completion_post_validate};
use crate::caps::resolve_completion_model;
use crate::completion_cache;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::privacy::{check_file_privacy, load_privacy_if_needed};
use crate::files_correction::canonical_path;
use crate::scratchpads;
use crate::at_commands::at_commands::AtCommandsContext;


const CODE_COMPLETION_TOP_N: usize = 5;

pub async fn handle_v1_code_completion(
    gcx: Arc<ARwLock<GlobalContext>>,
    code_completion_post: &mut CodeCompletionPost,
) -> Result<Response<Body>, ScratchError> {
    code_completion_post_validate(code_completion_post.clone())?;

    let cpath = canonical_path(&code_completion_post.inputs.cursor.file);
    check_file_privacy(load_privacy_if_needed(gcx.clone()).await, &cpath, &crate::privacy::FilePrivacyLevel::OnlySendToServersIControl)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let model_rec = resolve_completion_model(caps, &code_completion_post.model)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    if code_completion_post.parameters.max_new_tokens == 0 {
        code_completion_post.parameters.max_new_tokens = 50;
    }
    if code_completion_post.model == "" {
        code_completion_post.model = model_rec.base.id.clone();
    }
    info!("chosen completion model: {}, scratchpad: {}", code_completion_post.model, model_rec.scratchpad);
    code_completion_post.parameters.temperature = Some(code_completion_post.parameters.temperature.unwrap_or(0.2));
    let (cache_arc, tele_storage) = {
        let gcx_locked = gcx.write().await;
        (gcx_locked.completions_cache.clone(), gcx_locked.telemetry.clone())
    };
    if !code_completion_post.no_cache {
        let cache_key = completion_cache::cache_key_from_post(&code_completion_post);
        let cached_maybe = completion_cache::cache_get(cache_arc.clone(), cache_key.clone());
        if let Some(cached_json_value) = cached_maybe {
            // info!("cache hit for key {:?}", cache_key.clone());
            if !code_completion_post.stream {
                return crate::restream::cached_not_stream(&cached_json_value).await;
            } else {
                return crate::restream::cached_stream(&cached_json_value).await;
            }
        }
    }

    let ast_service_opt = gcx.read().await.ast_service.clone();
    let mut scratchpad = scratchpads::create_code_completion_scratchpad(
        gcx.clone(),
        &model_rec,
        &code_completion_post.clone(),
        cache_arc.clone(),
        tele_storage.clone(),
        ast_service_opt
    ).await.map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        model_rec.base.n_ctx,
        CODE_COMPLETION_TOP_N,
        true,
        vec![],
        "".to_string(),
        false,
    ).await));
    if !code_completion_post.stream {
        crate::restream::scratchpad_interaction_not_stream(ccx.clone(), &mut scratchpad, "completion".to_string(), &model_rec.base, &mut code_completion_post.parameters, false, None).await
    } else {
        crate::restream::scratchpad_interaction_stream(ccx.clone(), scratchpad, "completion-stream".to_string(), model_rec.base.clone(), code_completion_post.parameters.clone(), false, None).await
    }
}

pub async fn handle_v1_code_completion_web(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let mut code_completion_post = serde_json::from_slice::<CodeCompletionPost>(&body_bytes).map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    handle_v1_code_completion(gcx.clone(), &mut code_completion_post).await
}

pub async fn handle_v1_code_completion_prompt(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    // Almost the same function, but only returns the prompt (good for generating data)
    let mut post = serde_json::from_slice::<CodeCompletionPost>(&body_bytes).map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    code_completion_post_validate(post.clone())?;

    let cpath = canonical_path(&post.inputs.cursor.file);
    check_file_privacy(load_privacy_if_needed(gcx.clone()).await, &cpath, &crate::privacy::FilePrivacyLevel::OnlySendToServersIControl)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let model_rec = resolve_completion_model(caps, &post.model)
            .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

    // don't need cache, but go along
    let (cache_arc, tele_storage) = {
        let cx_locked = gcx.write().await;
        (cx_locked.completions_cache.clone(), cx_locked.telemetry.clone())
    };

    let ast_service_opt = gcx.read().await.ast_service.clone();
    let mut scratchpad = scratchpads::create_code_completion_scratchpad(
        gcx.clone(),
        &model_rec,
        &post,
        cache_arc.clone(),
        tele_storage.clone(),
        ast_service_opt
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;

    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        model_rec.base.n_ctx,
        CODE_COMPLETION_TOP_N,
        true,
        vec![],
        "".to_string(),
        false,
    ).await));
    let prompt = scratchpad.prompt(ccx.clone(), &mut post.parameters).await.map_err(|e|
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;

    let body = serde_json::json!({"prompt": prompt}).to_string();
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    return Ok(response);
}
