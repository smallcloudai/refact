use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};

use crate::call_validation::{ChatContent, ChatMessage, ChatPost};
use crate::caps::resolve_chat_model;
use crate::custom_error::ScratchError;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::git::checkpoints::create_workspace_checkpoint;
use crate::global_context::{GlobalContext, SharedGlobalContext};
use crate::indexing_utils::wait_for_indexing_if_needed;
use crate::integrations::docker::docker_container_manager::docker_container_check_status_or_start;
use crate::tools::tools_description::ToolDesc;
use crate::tools::tools_list::get_available_tools_by_chat_mode;

pub const CHAT_TOP_N: usize = 12;

pub async fn handle_v1_chat_completions(
    // standard openai-style handler
    Extension(gcx): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    _chat(gcx, &body_bytes, false).await
}

pub async fn handle_v1_chat(
    // less-standard openai-style handler that sends role="context_*" messages first, rewrites the user message
    Extension(gcx): Extension<SharedGlobalContext>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    _chat(gcx, &body_bytes, true).await
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

fn fill_sampling_params(chat_post: &mut ChatPost, n_ctx: usize, model_id: &str) {
    let mut max_tokens = if chat_post.increase_max_tokens {
        chat_post.max_tokens.unwrap_or(16384)
    } else {
        chat_post.max_tokens.unwrap_or(4096)
    };
    max_tokens = max_tokens.min(n_ctx / 4);
    chat_post.max_tokens = Some(max_tokens);
    if chat_post.parameters.max_new_tokens == 0 {
        chat_post.parameters.max_new_tokens = max_tokens;
    }
    chat_post.model = model_id.to_string();
    chat_post.parameters.n = chat_post.n;
    chat_post.parameters.temperature = Some(chat_post.parameters.temperature.unwrap_or(chat_post.temperature.unwrap_or(0.0)));
}


async fn _chat(
    gcx: Arc<ARwLock<GlobalContext>>,
    body_bytes: &hyper::body::Bytes,
    allow_at: bool
) -> Result<Response<Body>, ScratchError> {
    let mut chat_post: ChatPost = serde_json::from_slice::<ChatPost>(&body_bytes).map_err(|e| {
        tracing::warn!("chat handler cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;

    let inside_container = gcx.read().await.cmdline.inside_container;

    if chat_post.meta.chat_remote == inside_container {
        wait_for_indexing_if_needed(gcx.clone()).await;
    }

    let mut messages = deserialize_messages_from_post(&chat_post.messages)?;

    tracing::info!("chat_mode {:?}", chat_post.meta.chat_mode);

    let tools: Vec<ToolDesc> = get_available_tools_by_chat_mode(gcx.clone(), chat_post.meta.chat_mode).await
        .into_iter()
        .map(|tool| tool.tool_description())
        .collect();

    tracing::info!("tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let model_rec = resolve_chat_model(caps, &chat_post.model)
            .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    fill_sampling_params(&mut chat_post, model_rec.base.n_ctx, &model_rec.base.id);

    // extra validation to catch {"query": "Frog", "scope": "workspace"}{"query": "Toad", "scope": "workspace"}
    let re = regex::Regex::new(r"\{.*?\}").unwrap();
    for message in messages.iter_mut() {
        if !model_rec.supports_multimodality {
            if let ChatContent::Multimodal(content) = &message.content {
                if content.iter().any(|el| el.is_image()) {
                    return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("model '{}' does not support multimodality", model_rec.base.id)));
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

    let should_execute_remotely = chat_post.meta.chat_remote && !gcx.read().await.cmdline.inside_container;
    if should_execute_remotely {
        docker_container_check_status_or_start(gcx.clone(), &chat_post.meta.chat_id).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    let meta = if model_rec.base.support_metadata {
        Some(chat_post.meta.clone())
    } else {
        None
    };

    if chat_post.checkpoints_enabled {
        let latest_checkpoint = messages.iter().rev()
            .find(|msg| msg.role == "user" && !msg.checkpoints.is_empty())
            .and_then(|msg| msg.checkpoints.first().cloned());

        if let Some(latest_user_msg) = messages.last_mut().filter(|m| m.role == "user") {
            if chat_post.meta.chat_mode.supports_checkpoints() && latest_user_msg.checkpoints.is_empty() {
                match create_workspace_checkpoint(gcx.clone(), latest_checkpoint.as_ref(), &chat_post.meta.chat_id).await {
                    Ok((checkpoint, _)) => {
                        tracing::info!("Checkpoint created: {:?}", checkpoint);
                        latest_user_msg.checkpoints = vec![checkpoint];
                    },
                    Err(e) => tracing::error!("Failed to create checkpoint: {}", e),
                };
            }
        }
    }

    // SYSTEM PROMPT WAS HERE


    // chat_post.stream = Some(false);  // for debugging 400 errors that are hard to debug with streaming (because "data: " is not present and the error message is ignored by the library)
    let mut scratchpad = crate::scratchpads::create_chat_scratchpad(
        gcx.clone(),
        &mut chat_post,
        tools,
        &messages,
        true,
        &model_rec,
        allow_at,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    // if !chat_post.chat_id.is_empty() {
    //     let cache_dir = {
    //         let gcx_locked = gcx.read().await;
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
        gcx.clone(),
        model_rec.base.n_ctx,
        CHAT_TOP_N,
        false,
        messages.clone(),
        chat_post.meta.chat_id.clone(),
        should_execute_remotely,
        model_rec.base.id.clone(),
    ).await;
    ccx.subchat_tool_parameters = chat_post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = chat_post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    if chat_post.stream == Some(false) {
        crate::restream::scratchpad_interaction_not_stream(
            ccx_arc.clone(),
            &mut scratchpad,
            "chat".to_string(),
            &model_rec.base,
            &mut chat_post.parameters,
            chat_post.only_deterministic_messages,
            meta
        ).await
    } else {
        crate::restream::scratchpad_interaction_stream(
            ccx_arc.clone(),
            scratchpad,
            "chat-stream".to_string(),
            model_rec.base.clone(),
            chat_post.parameters.clone(),
            chat_post.only_deterministic_messages,
            meta
        ).await
    }
}
