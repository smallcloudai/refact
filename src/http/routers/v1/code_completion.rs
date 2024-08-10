use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::call_validation::{CodeCompletionPost, validate_post};
use crate::caps;
use crate::caps::CodeAssistantCaps;
use crate::completion_cache;
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::scratchpads;
use crate::at_commands::at_commands::AtCommandsContext;


const CODE_COMPLETION_TOP_N: usize = 5;

async fn _lookup_code_completion_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    code_completion_post: &CodeCompletionPost,
) -> Result<(String, String, serde_json::Value, usize), String> {
    let caps_locked = caps.read().unwrap();
    let (model_name, modelrec) =
        caps::which_model_to_use(
            &caps_locked.code_completion_models,
            &code_completion_post.model,
            &caps_locked.code_completion_default_model,
        )?;
    let (sname, patch) = caps::which_scratchpad_to_use(
        &modelrec.supports_scratchpads,
        &code_completion_post.scratchpad,
        &modelrec.default_scratchpad,
    )?;
    let caps_completion_n_ctx = caps_locked.code_completion_n_ctx;
    let mut n_ctx = modelrec.n_ctx;
    if caps_completion_n_ctx > 0 && n_ctx > caps_completion_n_ctx {
        // the model might be capable of a bigger context, but server (i.e. admin) tells us to use smaller (for example because latency)
        n_ctx = caps_completion_n_ctx;
    }
    Ok((model_name, sname.clone(), patch.clone(), n_ctx))
}

pub async fn handle_v1_code_completion(
    gcx: Arc<ARwLock<GlobalContext>>,
    code_completion_post: &mut CodeCompletionPost,
) -> Result<Response<Body>, ScratchError> {
    validate_post(code_completion_post.clone())?;
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let maybe = _lookup_code_completion_scratchpad(
        caps.clone(),
        &code_completion_post,
    ).await;
    if maybe.is_err() {
        // On error, this will also invalidate caps each 10 seconds, allows to overcome empty caps situation
        let _ = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 10).await;
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", maybe.unwrap_err())))
    }
    let (model_name, scratchpad_name, scratchpad_patch, n_ctx) = maybe.unwrap();
    if code_completion_post.parameters.max_new_tokens == 0 {
        code_completion_post.parameters.max_new_tokens = 50;
    }
    if code_completion_post.model == "" {
        code_completion_post.model = model_name.clone();
    }
    if code_completion_post.scratchpad == "" {
        code_completion_post.scratchpad = scratchpad_name.clone();
    }
    code_completion_post.parameters.temperature = Some(code_completion_post.parameters.temperature.unwrap_or(0.2));
    let (client1, api_key, cache_arc, tele_storage) = {
        let cx_locked = gcx.write().await;
        (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone(), cx_locked.completions_cache.clone(), cx_locked.telemetry.clone())
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

    let ast_module = gcx.read().await.ast_module.clone();
    let mut scratchpad = scratchpads::create_code_completion_scratchpad(
        gcx.clone(),
        caps,
        model_name.clone(),
        &code_completion_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        cache_arc.clone(),
        tele_storage.clone(),
        ast_module
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        n_ctx,
        CODE_COMPLETION_TOP_N,
        true,
        &vec![],
    ).await));
    if !code_completion_post.stream {
        crate::restream::scratchpad_interaction_not_stream(ccx.clone(), &mut scratchpad, "completion".to_string(), model_name, client1, api_key, &mut code_completion_post.parameters, false).await
    } else {
        crate::restream::scratchpad_interaction_stream(ccx.clone(), scratchpad, "completion-stream".to_string(), model_name, client1, api_key, code_completion_post.parameters.clone(), false).await
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
    validate_post(post.clone())?;
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let maybe = _lookup_code_completion_scratchpad(caps.clone(), &post).await;
    if maybe.is_err() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", maybe.unwrap_err())))
    }
    let (model_name, scratchpad_name, scratchpad_patch, n_ctx) = maybe.unwrap();

    // don't need cache, but go along
    let (cache_arc, tele_storage) = {
        let cx_locked = gcx.write().await;
        (cx_locked.completions_cache.clone(), cx_locked.telemetry.clone())
    };

    let ast_module = gcx.read().await.ast_module.clone();
    let mut scratchpad = scratchpads::create_code_completion_scratchpad(
        gcx.clone(),
        caps,
        model_name.clone(),
        &post,
        &scratchpad_name,
        &scratchpad_patch,
        cache_arc.clone(),
        tele_storage.clone(),
        ast_module
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;

    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        n_ctx,
        CODE_COMPLETION_TOP_N,
        true,
        &vec![],
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
