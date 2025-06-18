use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tracing::{error, info, warn};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::basic_utils::generate_random_hash;
use crate::call_validation::{ChatContent, ChatMessage, ChatToolCall, ContextEnum, ContextFile};
use crate::cloud::messages_req::ThreadMessage;
use crate::cloud::threads_req::{lock_thread, Thread};
use crate::cloud::threads_sub::{BasicStuff, ThreadPayload};
use crate::global_context::GlobalContext;
use crate::scratchpads::scratchpad_utils::max_tokens_for_rag_chat_by_tools;
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use crate::tools::tools_execute::pp_run_tools;


pub async fn match_against_confirm_deny(
    ccx: Arc<AMutex<AtCommandsContext>>,
    t_call: &ChatToolCall,
    tool: &mut Box<dyn Tool+Send>,
) -> Result<MatchConfirmDeny, String> {
    let args = match serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments) {
        Ok(args) => args,
        Err(e) => {
            return Err(format!("Tool use: couldn't parse arguments: {}. Error:\n{}", t_call.function.arguments, e));
        }
    };
    Ok(tool.match_against_confirm_deny(ccx.clone(), &args).await?)
}

pub async fn run_tool(
    ccx: Arc<AMutex<AtCommandsContext>>,
    t_call: &ChatToolCall,
    tool: &mut Box<dyn Tool+Send>,
) -> Result<(ChatMessage, Vec<ChatMessage>, Vec<ContextFile>), String> {
    let args = match serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments) {
        Ok(args) => args,
        Err(e) => {
            return Err(format!("Tool use: couldn't parse arguments: {}. Error:\n{}", t_call.function.arguments, e));
        }
    };
    
    match tool.match_against_confirm_deny(ccx.clone(), &args).await {
        Ok(res) => {
            match res.result {
                MatchConfirmDenyResult::DENY => {
                    let command_to_match = tool
                        .command_to_match_against_confirm_deny(ccx.clone(), &args).await
                        .unwrap_or("<error_command>".to_string());
                    return Err(format!("tool use: command '{command_to_match}' is denied"));
                }
                _ => {}
            }
        }
        Err(err) => return Err(err),
    };

    let tool_execute_results = match tool.tool_execute(ccx.clone(), &t_call.id.to_string(), &args).await {
        Ok((_, mut tool_execute_results)) => {
            for tool_execute_result in &mut tool_execute_results {
                if let ContextEnum::ChatMessage(m) = tool_execute_result {
                    m.tool_failed = Some(false);
                }
            }
            tool_execute_results
        }
        Err(e) => {
            return Err(e);
        }
    };

    let (mut tool_result_mb, mut other_messages, mut context_files) = (None, vec![], vec![]);
    for msg in tool_execute_results {
        match msg {
            ContextEnum::ChatMessage(m) => {
                if !m.tool_call_id.is_empty() {
                    if tool_result_mb.is_some() {
                        return Err(format!("duplicated output message from the tool: {}", t_call.function.name));
                    }
                    tool_result_mb = Some(m);
                } else {
                    other_messages.push(m);
                }
            },
            ContextEnum::ContextFile(m) => {
                context_files.push(m);
            }
        }
    }
    let tool_result = match tool_result_mb {
        Some(m) => m,
        None => return Err(format!("tool use: failed to get output message from tool: {}", t_call.function.name)),
    };
    
    Ok((tool_result, other_messages, context_files))
}

async fn initialize_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    ft_fexp_id: &str,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
    api_key: String,
    located_fgroup_id: String
) -> Result<(), String> {
    let expert = crate::cloud::experts_req::get_expert(api_key.clone(), ft_fexp_id).await?;
    let cloud_tools = crate::cloud::cloud_tools_req::get_cloud_tools(api_key.clone(), &located_fgroup_id).await?;
    info!("retrieving cloud tools for thread `{}`: {:?}", thread.ft_id, cloud_tools);
    let last_message = thread_messages.iter()
        .max_by_key(|x| x.ftm_num)
        .ok_or("No last message found".to_string())
        .clone()?;
    let tools: Vec<Box<dyn Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone())
            .await
            .into_iter()
            .filter(|tool| expert.is_tool_allowed(&tool.tool_description().name))
            .collect();
    let tool_names = tools.iter().map(|x| x.tool_description().name.clone()).collect::<Vec<_>>();
    let mut tool_descriptions: Vec<_> = tools
        .iter()
        .map(|x| x.tool_description().into_openai_style())
        .collect();
    tool_descriptions.extend(
        cloud_tools.into_iter()
            .filter(|x| expert.is_tool_allowed(&x.ctool_name))
            .filter(|x| {
                if tool_names.contains(&x.ctool_name) {
                    error!("tool `{}` is already in the toolset, filtering it out. This might cause races between cloud and binary", x.ctool_name);
                    false
                } else { true }
            })
            .map(|x| x.into_openai_style())
    );
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
    let last_tool_calls = thread_messages.iter()
        .rev()
        .find(|x| x.ftm_role == "assistant" && x.ftm_tool_calls.is_some())
        .cloned()
        .map(|x| 
            crate::cloud::messages_req::convert_thread_messages_to_messages(&vec![x.clone()])[0].clone()
        )
        .map(|x| x.tool_calls.clone().expect("checked before"))
        .ok_or("No last assistant message with tool calls found".to_string())?;
    let (alt, prev_alt) = thread_messages
        .last()
        .map(|msg| (msg.ftm_alt, msg.ftm_prev_alt))
        .unwrap_or((0, 0));
    let messages = crate::cloud::messages_req::convert_thread_messages_to_messages(thread_messages)
        .into_iter()
        .filter(|x| x.role != "kernel")
        .collect::<Vec<_>>();
    let ccx = Arc::new(AMutex::new(
        AtCommandsContext::new(gcx.clone(), n_ctx, top_n, false, messages.clone(),
            thread.ft_id.to_string(), false
        ).await,
    ));
    let toolset = thread.ft_toolset.clone().unwrap_or_default();
    let allowed_tools = crate::cloud::messages_req::get_tool_names_from_openai_format(&toolset).await?;
    let mut all_tools: IndexMap<String, Box<dyn Tool + Send>> =
        crate::tools::tools_list::get_available_tools(gcx.clone()).await
            .into_iter()
            .filter(|x| allowed_tools.contains(&x.tool_description().name))
            .map(|x| (x.tool_description().name, x))
            .collect();
    let mut all_tool_output_messages = vec![];
    let mut all_context_files = vec![];
    let mut all_other_messages = vec![];
    let mut tool_id_to_index = HashMap::new();
    // Default tokens limit for tools that perform internal compression (`tree()`, ...)
    ccx.lock().await.tokens_for_rag = max_new_tokens;

    let confirmed_tool_call_ids: Vec<String> = if let Some(confirmation_response) = &thread.ft_confirmation_response {
        serde_json::from_value(confirmation_response.clone()).map_err(|err| {
            format!("error parsing confirmation response: {}", err)
        })?
    } else { vec![] };
    let mut required_confirmation = vec![];
    for (idx, t_call) in last_tool_calls.iter().enumerate() {
        let is_answered = thread_messages.iter()
            .filter(|x| x.ftm_role == "tool")
            .any(|x| t_call.id == x.ftm_call_id);
        if is_answered {
            warn!("tool use: tool call `{}` is already answered, skipping it", t_call.id);
            continue;
        }

        let tool = match all_tools.get_mut(&t_call.function.name) {
            Some(tool) => tool,
            None => {
                warn!("tool use: function {:?} not found", &t_call.function.name);
                continue;
            }
        };
        let skip_confirmation = confirmed_tool_call_ids.contains(&t_call.id);
        if !skip_confirmation {
            let confirm_deny_res = match_against_confirm_deny(ccx.clone(), t_call, tool).await?;
            match &confirm_deny_res.result {
                MatchConfirmDenyResult::CONFIRMATION => {
                    info!("tool use: tool call `{}` requires confirmation, skipping it", t_call.id);
                    required_confirmation.push(json!({
                        "tool_call_id": t_call.id,
                        "command": confirm_deny_res.command,
                        "rule": confirm_deny_res.rule,
                        "ftm_num": last_message.ftm_num + 1 + (idx as i32),
                    }));
                    continue;
                }
                _ => { }
            }
        } else {
            info!("tool use: tool call `{}` is confirmed, processing to call it", t_call.id);
        }
        
        let (tool_result, other_messages, context_files) = match run_tool(ccx.clone(), t_call, tool).await {
            Ok(res) => res,
            Err(err) => {
                warn!("tool use: failed to run tool: {}", err);
                (
                    ChatMessage {
                        role: "tool".to_string(),
                        content: ChatContent::SimpleText(err),
                        tool_call_id: t_call.id.clone(),
                        ..ChatMessage::default()
                    }, vec![], vec![]
                )
            }
        };
        all_tool_output_messages.push(tool_result);
        all_context_files.extend(context_files);
        all_other_messages.extend(other_messages);
        tool_id_to_index.insert(t_call.id.clone(), last_message.ftm_num + 1 + (idx as i32));
    }
    
    let reserve_for_context = max_tokens_for_rag_chat_by_tools(&last_tool_calls, &all_context_files, n_ctx, max_new_tokens);
    ccx.lock().await.tokens_for_rag = reserve_for_context;
    let (generated_tool, generated_other) = pp_run_tools(
        ccx.clone(), &vec![], false,
        all_tool_output_messages, all_other_messages, &mut all_context_files, reserve_for_context,
        None, &None,
    ).await;
    let mut afterwards_index = last_message.ftm_num + last_tool_calls.len() as i32 + 1;
    let mut all_output_messages = vec![];
    for msg in generated_tool.into_iter().chain(generated_other.into_iter()) {
        let index = if let Some(index) = tool_id_to_index.get(&msg.tool_call_id) {
            index.clone()
        } else {
            afterwards_index += 1;
            afterwards_index - 1
        };
        let output_thread_messages = crate::cloud::messages_req::convert_messages_to_thread_messages(
            vec![msg], alt, prev_alt, index, &thread.ft_id, last_message.ftm_user_preferences.clone(),
        )?;
        all_output_messages.extend(output_thread_messages);
    }

    if !required_confirmation.is_empty() {
        if !crate::cloud::threads_req::set_thread_confirmation_request(
            api_key.clone(), &thread.ft_id, serde_json::to_value(required_confirmation.clone()).unwrap()
        ).await? {
            warn!("tool use: cannot set confirmation requests: {:?}", required_confirmation);
        }
    }

    if !all_output_messages.is_empty() {
        crate::cloud::messages_req::create_thread_messages(api_key, &thread.ft_id, all_output_messages).await?;
    } else {
        info!("thread `{}` has no tool output messages. Skipping it", thread.ft_id);
    }
    Ok(())
}

pub async fn process_thread_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload,
    basic_info: &BasicStuff,
    api_key: String,
    app_searchable_id: String,
    located_fgroup_id: String,
) -> Result<(), String> {
    if thread_payload.ft_need_tool_calls == -1 
        || thread_payload.owner_fuser_id != basic_info.fuser_id 
        || !thread_payload.ft_locked_by.is_empty() {
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

    let hash = generate_random_hash(16);
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
        api_key.clone(),
        located_fgroup_id
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
    located_fgroup_id: String
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
        if thread_payload.ft_fexp_id.is_none() {
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
        let ft_fexp_id = thread.ft_fexp_id.clone().expect("checked before");
        info!("initializing system prompt for thread `{}`", thread_id);
        initialize_thread(gcx.clone(), &ft_fexp_id, &thread, &messages, api_key.clone(), located_fgroup_id).await
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
