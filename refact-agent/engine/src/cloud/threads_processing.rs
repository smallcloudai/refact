use std::collections::HashSet;
use std::sync::Arc;
use indexmap::IndexMap;

use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use serde_json::json;
use tracing::{error, info};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ChatContent;
use crate::cloud::messages_req::ThreadMessage;
use crate::cloud::threads_req::{lock_thread, Thread};
use crate::cloud::threads_sub::{BasicStuff, ThreadPayload};
use crate::global_context::GlobalContext;


async fn initialize_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    expert_name: &str,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
    api_key: String
) -> Result<(), String> {
    let expert = crate::cloud::experts_req::get_expert(api_key.clone(), expert_name).await?;
    let last_message = thread_messages.iter()
        .max_by_key(|x| x.ftm_num)
        .ok_or("No last message found".to_string())
        .clone()?;
    let tools: Vec<Box<dyn crate::tools::tools_description::Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone())
            .await
            .into_iter()
            .filter(|tool| expert.is_tool_allowed(&tool.tool_description().name))
            .collect();
    let tool_descriptions = tools
        .iter()
        .map(|x| x.tool_description().into_openai_style())
        .collect::<Vec<_>>();
    crate::cloud::threads_req::set_thread_toolset(api_key.clone(), &thread.ft_id, tool_descriptions).await?;
    let updated_system_prompt = crate::scratchpads::chat_utils_prompts::system_prompt_add_extra_instructions(
        gcx.clone(), expert.fexp_system_prompt.clone(), HashSet::new()
    ).await;
    let output_thread_messages = vec![ThreadMessage {
        ftm_belongs_to_ft_id: last_message.ftm_belongs_to_ft_id.clone(),
        ftm_alt: last_message.ftm_alt.clone(),
        ftm_num: 0,
        ftm_prev_alt: 100,
        ftm_role: "system".to_string(),
        ftm_content: Some(
            serde_json::to_value(ChatContent::SimpleText(updated_system_prompt)).unwrap(),
        ),
        ftm_tool_calls: None,
        ftm_call_id: "".to_string(),
        ftm_usage: None,
        ftm_created_ts: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        ftm_provenance: json!({"system_type": "refact_lsp", "version": env!("CARGO_PKG_VERSION") }),
        ftm_user_preferences: last_message.ftm_user_preferences.clone(),
    }];
    crate::cloud::messages_req::create_thread_messages(
        api_key,
        &thread.ft_id,
        output_thread_messages,
    ).await?;
    Ok(())
}

async fn call_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
    api_key: String
) -> Result<(), String> {
    // TODO: think of better ways to handle these params
    let n_ctx = 128000;
    let top_n = 12;
    let max_new_tokens = 8192;

    let last_message = thread_messages.iter()
        .max_by_key(|x| x.ftm_num)
        .ok_or("No last message found".to_string())
        .clone()?;
    let (alt, prev_alt) = thread_messages
        .last()
        .map(|msg| (msg.ftm_alt, msg.ftm_prev_alt))
        .unwrap_or((0, 0));
    let messages = crate::cloud::messages_req::convert_thread_messages_to_messages(thread_messages)
        .into_iter()
        .filter(|x| x.role != "kernel")
        .collect::<Vec<_>>();
    let ccx = Arc::new(AMutex::new(
        AtCommandsContext::new(
            gcx.clone(),
            n_ctx,
            top_n,
            false,
            messages.clone(),
            thread.ft_id.to_string(),
            false,
            // Some(current_model)
            Some("refact/gpt-4.1-mini".to_string())
        ).await,
    ));
    let toolset = thread.ft_toolset.clone().unwrap_or_default();
    let allowed_tools = crate::cloud::messages_req::get_tool_names_from_openai_format(&toolset).await?;
    let mut all_tools: IndexMap<String, Box<dyn crate::tools::tools_description::Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone()).await
            .into_iter()
            .filter(|x| allowed_tools.contains(&x.tool_description().name))
            .map(|x| (x.tool_description().name, x))
            .collect();
    let mut has_rag_results = crate::scratchpads::scratchpad_utils::HasRagResults::new();
    let messages_count = messages.len();
    let (output_messages, _) = crate::tools::tools_execute::run_tools_locally(
        ccx.clone(),
        &mut all_tools,
        None,
        max_new_tokens,
        &messages,
        &mut has_rag_results,
        &None,
    ).await?;
    if messages.len() == output_messages.len() {
        tracing::warn!(
            "Thread has no active tool call awaiting but still has need_tool_call turned on"
        );
        return Ok(());
    }
    let output_thread_messages = crate::cloud::messages_req::convert_messages_to_thread_messages(
        output_messages.into_iter().skip(messages_count).collect(),
        alt,
        prev_alt,
        last_message.ftm_num + 1,
        &thread.ft_id,
        last_message.ftm_user_preferences.clone(),
    )?;
    crate::cloud::messages_req::create_thread_messages(
        api_key,
        &thread.ft_id,
        output_thread_messages,
    ).await?;
    Ok(())
}


pub async fn process_thread_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload,
    basic_info: &BasicStuff,
    api_key: String,
    app_searchable_id: String
) -> Result<(), String> {
    if thread_payload.ft_need_tool_calls == -1 
        || thread_payload.owner_fuser_id != basic_info.fuser_id 
        || thread_payload.ft_locked_by.is_empty() {
        return Ok(());
    }
    if let Some(ft_app_searchable) = thread_payload.ft_app_searchable.clone() {
        if ft_app_searchable != app_searchable_id {
            info!("thread `{}` has different `app_searchable` id, skipping it: {} != {}", 
                thread_payload.ft_id, app_searchable_id, ft_app_searchable
            );
            return Ok(());
        }
    } else {
        info!("thread `{}` doesn't have the `app_searchable` id, skipping it", thread_payload.ft_id);
        return Ok(());
    }
    if let Some(error) = thread_payload.ft_error.as_ref() {
        info!("thread `{}` has the error: `{}`. Skipping it", thread_payload.ft_id, error);
        return Ok(());
    }

    let hash = crate::cloud::threads_sub::generate_random_hash(16);
    let thread_id = thread_payload.ft_id.clone();
    let lock_result = lock_thread(api_key.clone(), &thread_id, &hash).await;
    if let Err(err) = lock_result {
        info!("failed to lock thread `{}` with hash `{}`: {}", thread_id, hash, err);
        return Ok(());
    }
    info!("thread `{}` locked successfully with hash `{}`", thread_id, hash);
    let process_result = process_locked_thread(
        gcx, 
        thread_payload, 
        &thread_id, 
        api_key.clone()
    ).await;
    match crate::cloud::threads_req::unlock_thread(api_key.clone(), thread_id.clone(), hash).await {
        Ok(_) => info!("thread `{}` unlocked successfully", thread_id),
        Err(err) => {
            error!("failed to unlock thread `{}`: {}", thread_id, err);
        },
    }
    process_result
}

async fn process_locked_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload,
    thread_id: &str,
    api_key: String,
) -> Result<(), String> {
    let messages = match crate::cloud::messages_req::get_thread_messages(
        api_key.clone(),
        thread_id,
        thread_payload.ft_need_tool_calls,
    ).await {
        Ok(msgs) => msgs,
        Err(e) => {
            return Err(e);
        }
    };
    if messages.is_empty() {
        info!("thread `{}` has no messages. Skipping it", thread_id);
        return Ok(());
    }
    let thread = match crate::cloud::threads_req::get_thread(api_key.clone(), thread_id).await {
        Ok(t) => t,
        Err(e) => {
            return Err(e);
        }
    };
    let need_to_append_system = messages.iter().all(|x| x.ftm_role != "system");
    if need_to_append_system {
        if thread_payload.ft_fexp_name.is_none() {
            info!("thread `{}` has no expert set. Skipping it", thread_id);
            return Ok(());
        }
    } else {
        if thread.ft_toolset.is_none() {
            info!("thread `{}` has no toolset. Skipping it", thread_id);
            return Ok(());
        }
    }
    let result = if need_to_append_system {
        let exp_name = thread.ft_fexp_name.clone().expect("checked before");
        info!("initializing system prompt for thread `{}`", thread_id);
        initialize_thread(gcx.clone(), &exp_name, &thread, &messages, api_key.clone()).await
    } else {
        info!("calling tools for thread `{}`", thread_id);
        call_tools(gcx.clone(), &thread, &messages, api_key.clone()).await
    };
    if let Err(err) = &result {
        info!("failed to process thread `{}`, setting error: {}", thread_id, err);
        if let Err(set_err) = crate::cloud::threads_req::set_error_thread(
            api_key.clone(), 
            thread_id.to_string(), 
            err.clone()
        ).await {
            return Err(format!("Failed to set error on thread: {}", set_err));
        }
    }
    result
}
