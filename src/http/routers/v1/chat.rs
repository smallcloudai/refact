use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::Mutex as AMutex;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use tracing::info;

use crate::call_validation::{ChatContent, ChatMessage, ChatPost};
use crate::caps::CodeAssistantCaps;
use crate::custom_error::ScratchError;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::global_context::SharedGlobalContext;
use crate::integrations::docker::docker_container_manager::docker_container_check_status_or_start;
use crate::{caps, scratchpads};


pub const CHAT_TOP_N: usize = 7;

pub async fn lookup_chat_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    chat_post: &ChatPost,
) -> Result<(String, String, serde_json::Value, usize, bool, bool), String> {
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
    Ok((
        model_name,
        sname.clone(),
        patch.clone(),
        recommended_model_record.n_ctx,
        recommended_model_record.supports_tools,
        recommended_model_record.supports_multimodality,
    ))
}

pub async fn handle_v1_chat_completions(
    // standard openai-style handler
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    chat(global_context, body_bytes, false).await
}

pub async fn handle_v1_chat(
    // less-standard openai-style handler that sends role="context_*" messages first, rewrites the user message
    Extension(global_context): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    chat(global_context, body_bytes, true).await
}

pub fn deserialize_messages_from_post(messages: &Vec<serde_json::Value>) -> Result<Vec<ChatMessage>, ScratchError> {
    let messages: Vec<ChatMessage> = messages.iter()
        .map(|x| serde_json::from_value(x.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            tracing::error!("can't deserialize ChatMessage: {}", e);
            ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
        })?;
    Ok(messages)
}

async fn chat(
    global_context: SharedGlobalContext,
    body_bytes: hyper::body::Bytes,
    allow_at: bool,
) -> Result<Response<Body>, ScratchError> {
    let mut chat_post = serde_json::from_slice::<ChatPost>(&body_bytes).map_err(|e| {
        info!("chat handler cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let mut messages = deserialize_messages_from_post(&chat_post.messages)?;

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await?;
    let (model_name, scratchpad_name, scratchpad_patch, n_ctx, supports_tools, supports_multimodality) = lookup_chat_scratchpad(
        caps.clone(),
        &chat_post,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e))
    })?;
    if chat_post.parameters.max_new_tokens == 0 {
        chat_post.parameters.max_new_tokens = chat_post.max_tokens;
    }
    if chat_post.parameters.max_new_tokens == 0 {
        chat_post.parameters.max_new_tokens = 1024;
    }
    chat_post.parameters.n = chat_post.n;
    chat_post.parameters.temperature = Some(chat_post.parameters.temperature.unwrap_or(chat_post.temperature.unwrap_or(0.2)));
    chat_post.model = model_name.clone();

    // extra validation to catch {"query": "Frog", "scope": "workspace"}{"query": "Toad", "scope": "workspace"}
    let re = regex::Regex::new(r"\{.*?\}").unwrap();
    for message in messages.iter_mut() {
        if !supports_multimodality {
            if let ChatContent::Multimodal(content) = &message.content {
                if content.iter().any(|el| el.is_image()) {
                    return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("model '{}' does not support multimodality", model_name)));
                }
            }
            message.content = ChatContent::SimpleText(message.content.content_text_only());
        }

        if let Some(tool_calls) = &mut message.tool_calls {
            for call in tool_calls {
                let args_input = &call.function.arguments;
                let will_it_work: Result<serde_json::Value, _> = serde_json::from_str(args_input);
                if will_it_work.is_ok() {
                    continue;
                }
                tracing::warn!("Failed to parse tool call arguments: {}", will_it_work.err().unwrap());
                let args_corrected_json: serde_json::Value = if let Some(captures) = re.captures(args_input) {
                    let corrected_arg = captures.get(0).unwrap().as_str();
                    tracing::warn!("Invalid JSON found in tool call arguments; using corrected string: {}", corrected_arg);
                    match serde_json::from_str(corrected_arg) {
                        Ok(value) => value,
                        Err(e) => {
                            tracing::warn!("Failed to parse corrected tool call arguments: {}", e);
                            continue;
                        }
                    }
                } else {
                    tracing::warn!("No valid JSON found in tool call arguments.");
                    continue;
                };
                if let Ok(args_corrected) = serde_json::to_string(&args_corrected_json) {
                    tracing::warn!("Correcting tool call arguments from {:?} to {:?}", args_input, args_corrected);
                    call.function.arguments = args_corrected;  // <-------------------------------------------------- correction is saved here
                } else {
                    tracing::warn!("Failed to serialize corrected tool call arguments.");
                }
            }
        }
    }

    // chat_post.stream = Some(false);  // for debugging 400 errors that are hard to debug with streaming (because "data: " is not present and the error message is ignored by the library)
    let mut scratchpad = scratchpads::create_chat_scratchpad(
        global_context.clone(),
        caps,
        model_name.clone(),
        &chat_post,
        &messages,
        &scratchpad_name,
        &scratchpad_patch,
        allow_at,
        supports_tools,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    // if !chat_post.chat_id.is_empty() {
    //     let cache_dir = {
    //         let gcx_locked = global_context.read().await;
    //         gcx_locked.cache_dir.clone()
    //     };
    //     let notes_dir_path = cache_dir.join("chats");
    //     let _ = std::fs::create_dir_all(&notes_dir_path);
    //     let notes_path = notes_dir_path.join(format!("chat{}_{}.json",
    //         chrono::Local::now().format("%Y%m%d"),
    //         chat_post.chat_id,
    //     ));
    //     let _ = std::fs::write(&notes_path, serde_json::to_string_pretty(&chat_post.messages).unwrap());
    // }
    let mut ccx = AtCommandsContext::new(
        global_context.clone(),
        n_ctx,
        CHAT_TOP_N,
        false,
        messages.clone(),
        chat_post.chat_id.clone(),
    ).await;
    ccx.subchat_tool_parameters = chat_post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = chat_post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    let is_inside_container = global_context.read().await.cmdline.inside_container;
    let run_chat_threads_inside_container = ccx_arc.lock().await.docker_tool.clone()
        .map(|docker_tool| docker_tool.integration_docker.run_chat_threads_inside_container)
        .unwrap_or(false);

    if run_chat_threads_inside_container && !is_inside_container {
        docker_container_check_status_or_start(ccx_arc.clone()).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    if chat_post.stream.is_some() && !chat_post.stream.unwrap() {
        crate::restream::scratchpad_interaction_not_stream(
            ccx_arc.clone(),
            &mut scratchpad,
            "chat".to_string(),
            model_name,
            &mut chat_post.parameters,
            chat_post.only_deterministic_messages,
        ).await
    } else {
        crate::restream::scratchpad_interaction_stream(
            ccx_arc.clone(),
            scratchpad,
            "chat-stream".to_string(),
            model_name,
            chat_post.parameters.clone(),
            chat_post.only_deterministic_messages,
        ).await
    }
}
