use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use axum::Extension;
use axum::response::Result;
use hyper::{Body, Response, StatusCode};
use serde_json::Value;

use crate::call_validation::{ChatContent, ChatMessage, ChatPost, ChatMode};
use crate::caps::CodeAssistantCaps;
use crate::custom_error::ScratchError;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::git::checkpoints::create_workspace_checkpoint;
use crate::global_context::{is_metadata_supported, GlobalContext, SharedGlobalContext};
use crate::integrations::docker::docker_container_manager::docker_container_check_status_or_start;


pub fn available_tools_by_chat_mode(current_tools: Vec<Value>, chat_mode: &ChatMode) -> Vec<Value> {
    fn filter_out_tools(current_tools: &Vec<Value>, blacklist: &Vec<&str>) -> Vec<Value> {
        current_tools
            .into_iter()
            .filter(|x| {
                x.get("function")
                    .and_then(|x| x.get("name"))
                    .and_then(|tool_name| tool_name.as_str())
                    .map(|tool_name_str| !blacklist.contains(&tool_name_str))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }
    fn keep_tools(current_tools: &Vec<Value>, whitelist: &Vec<&str>) -> Vec<Value> {
        current_tools
            .into_iter()
            .filter(|x| {
                x.get("function")
                    .and_then(|x| x.get("name"))
                    .and_then(|tool_name| tool_name.as_str())
                    .map(|tool_name_str| whitelist.contains(&tool_name_str))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
    match chat_mode {
        ChatMode::NO_TOOLS => {
            vec![]
        },
        ChatMode::EXPLORE | ChatMode::AGENT => filter_out_tools(&current_tools, &vec!["think"]),
        ChatMode::THINKING_AGENT => filter_out_tools(&current_tools, &vec!["knowledge"]),
        ChatMode::CONFIGURE => filter_out_tools(&current_tools, &vec!["tree", "locate", "knowledge", "search"]),
        ChatMode::PROJECT_SUMMARY => keep_tools(&current_tools, &vec!["cat", "tree", "bash"]),
    }
}

pub const CHAT_TOP_N: usize = 7;

pub async fn lookup_chat_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    chat_post: &ChatPost,
) -> Result<(String, String, serde_json::Value, usize, bool, bool, bool), String> {
    let caps_locked = caps.read().unwrap();
    let (model_name, recommended_model_record) =
        crate::caps::which_model_to_use(
            &caps_locked.code_chat_models,
            &chat_post.model,
            &caps_locked.code_chat_default_model,
        )?;
    let (sname, patch) = crate::caps::which_scratchpad_to_use(
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
        recommended_model_record.supports_clicks,
    ))
}

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

async fn _chat(
    gcx: Arc<ARwLock<GlobalContext>>,
    body_bytes: &hyper::body::Bytes,
    allow_at: bool
) -> Result<Response<Body>, ScratchError> {
    let mut chat_post: ChatPost = serde_json::from_slice::<ChatPost>(&body_bytes).map_err(|e| {
        tracing::warn!("chat handler cannot parse input:\n{:?}", body_bytes);
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let mut messages = deserialize_messages_from_post(&chat_post.messages)?;

    tracing::info!("chat_mode {:?}", chat_post.meta.chat_mode);

    if chat_post.meta.chat_mode == ChatMode::NO_TOOLS {
        chat_post.tools = None;
    } else {
        if let Some(tools) = &mut chat_post.tools {
            for tool in &mut *tools {
                if let Some(function) = tool.get_mut("function") {
                    function.as_object_mut().unwrap().remove("agentic");
                }
            }
            chat_post.tools = Some(available_tools_by_chat_mode(tools.clone(), &chat_post.meta.chat_mode));
        } else {
            // TODO at some point, get rid of /tools call on client, make so we can have chat_post.tools==None and just fill the tools here
            chat_post.tools = Some(available_tools_by_chat_mode(vec![], &chat_post.meta.chat_mode));
        }
        tracing::info!("tools [{}]", chat_post.tools.as_ref().map_or("".to_string(), |tools| {
            tools.iter()
                .filter_map(|tool| tool.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()))
                .collect::<Vec<&str>>()
                .join(", ")
        }));
    }

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await?;
    let (model_name, scratchpad_name, scratchpad_patch, n_ctx, supports_tools, supports_multimodality, supports_clicks) = lookup_chat_scratchpad(
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
    chat_post.parameters.temperature = Some(chat_post.parameters.temperature.unwrap_or(chat_post.temperature.unwrap_or(0.0)));
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

    let should_execute_remotely = chat_post.meta.chat_remote && !gcx.read().await.cmdline.inside_container;
    if should_execute_remotely {
        docker_container_check_status_or_start(gcx.clone(), &chat_post.meta.chat_id).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    let meta = {
        if is_metadata_supported(gcx.clone()).await {
            Some(chat_post.meta.clone())
        } else {
            None
        }
    };
    
    if chat_post.checkpoints_enabled {
        let latest_checkpoint = messages.iter().rev()
            .find(|msg| msg.role == "user" && !msg.checkpoints.is_empty())
            .and_then(|msg| msg.checkpoints.first().cloned());

        if let Some(latest_user_msg) = messages.last_mut().filter(|m| m.role == "user") {
            if [ChatMode::AGENT, ChatMode::CONFIGURE, ChatMode::PROJECT_SUMMARY].contains(&chat_post.meta.chat_mode) 
                && latest_user_msg.checkpoints.is_empty() {
                match create_workspace_checkpoint(gcx.clone(), latest_checkpoint.as_ref(), &chat_post.meta.chat_id).await {
                    Ok((checkpoint, _, _)) => {
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
        caps,
        model_name.clone(),
        &mut chat_post,
        &messages,
        true,
        &scratchpad_name,
        &scratchpad_patch,
        allow_at,
        supports_tools,
        supports_clicks,
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
        n_ctx,
        CHAT_TOP_N,
        false,
        messages.clone(),
        chat_post.meta.chat_id.clone(),
        should_execute_remotely,
    ).await;
    ccx.subchat_tool_parameters = chat_post.subchat_tool_parameters.clone();
    ccx.postprocess_parameters = chat_post.postprocess_parameters.clone();
    let ccx_arc = Arc::new(AMutex::new(ccx));

    if chat_post.stream.is_some() && !chat_post.stream.unwrap() {
        crate::restream::scratchpad_interaction_not_stream(
            ccx_arc.clone(),
            &mut scratchpad,
            "chat".to_string(),
            model_name,
            &mut chat_post.parameters,
            chat_post.only_deterministic_messages,
            meta
        ).await
    } else {
        crate::restream::scratchpad_interaction_stream(
            ccx_arc.clone(),
            scratchpad,
            "chat-stream".to_string(),
            model_name,
            chat_post.parameters.clone(),
            chat_post.only_deterministic_messages,
            meta
        ).await
    }
}
