use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
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
use crate::cloud::threads_sub::{BasicStuff, ThreadMessagePayload, ThreadPayload};
use crate::git::checkpoints::create_workspace_checkpoint;
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
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str
) -> Result<(), String> {
    let expert = crate::cloud::experts_req::get_expert(cmd_address_url, api_key, ft_fexp_id).await?;
    let cloud_tools = crate::cloud::cloud_tools_req::get_cloud_tools(cmd_address_url, api_key, located_fgroup_id).await?;
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
    crate::cloud::threads_req::set_thread_toolset(cmd_address_url, api_key, &thread.ft_id, tool_descriptions).await?;
    let updated_system_prompt = crate::scratchpads::chat_utils_prompts::system_prompt_add_extra_instructions(
        gcx.clone(), expert.fexp_system_prompt.clone(), HashSet::new()
    ).await;
    let output_thread_messages = vec![ThreadMessage {
        ftm_belongs_to_ft_id: thread.ft_id.clone(),
        ftm_alt: 100,  // convention, system prompt always at num=0, alt=100
        ftm_num: 0,
        ftm_prev_alt: 100,
        ftm_role: "system".to_string(),
        ftm_content: Some(
            serde_json::to_value(ChatContent::SimpleText(updated_system_prompt)).unwrap(),
        ),
        ftm_tool_calls: None,
        ftm_call_id: "".to_string(),
        ftm_usage: None,
        ftm_created_ts: std::time::SystemTime::now()    // XXX not accepted by server
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        ftm_provenance: json!({"system_type": "refact_lsp", "version": env!("CARGO_PKG_VERSION") }),
        ftm_user_preferences: None,
    }];
    crate::cloud::messages_req::create_thread_messages(
        &cmd_address_url,
        &api_key,
        &thread.ft_id,
        output_thread_messages,
    ).await?;
    Ok(())
}

async fn call_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread: &Thread,
    thread_messages: &Vec<ThreadMessage>,
    alt: i64,
    cmd_address_url: &str,
    api_key: &str
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
    let mut tool_id_to_num = HashMap::new();
    let mut required_confirmation = vec![];
    // Default tokens limit for tools that perform internal compression (`tree()`, ...)
    ccx.lock().await.tokens_for_rag = max_new_tokens;

    let confirmed_tool_call_ids: Vec<String> = if let Some(confirmation_response) = &thread.ft_confirmation_response {
        if confirmation_response.as_str().unwrap_or("") == "*" {
            last_tool_calls.iter().map(|x| x.id.clone()).collect()
        } else {
            serde_json::from_value(confirmation_response.clone()).map_err(|err| {
                format!("error parsing confirmation response: {}", err)
            })?
        }
    } else { vec![] };
    let waiting_for_confirmation = if let Some(confirmation_response) = &thread.ft_confirmation_response {
        match serde_json::from_value::<Vec<serde_json::Value>>(confirmation_response.clone()) {
            Ok(items) => {
                items.iter()
                    .filter_map(|item| item.get("tool_call_id").and_then(|id| id.as_str()))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            }
            Err(err) => {
                warn!("error parsing confirmation response: {}", err);
                vec![]
            }
        }
    } else { vec![] };
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
                    if !waiting_for_confirmation.contains(&t_call.id) {
                        required_confirmation.push(json!({
                            "tool_call_id": t_call.id,
                            "command": confirm_deny_res.command,
                            "rule": confirm_deny_res.rule,
                            "ftm_num": last_message.ftm_num + 1 + idx as i64,
                        }));
                    }
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
        tool_id_to_num.insert(t_call.id.clone(), last_message.ftm_num + 1 + idx as i64);
    }

    let reserve_for_context = max_tokens_for_rag_chat_by_tools(&last_tool_calls, &all_context_files, n_ctx, max_new_tokens);
    ccx.lock().await.tokens_for_rag = reserve_for_context;
    let (generated_tool, generated_other) = pp_run_tools(
        ccx.clone(), &vec![], false, all_tool_output_messages,
        all_other_messages, &mut all_context_files, reserve_for_context, &None,
    ).await;
    let mut afterwards_num = last_message.ftm_num + last_tool_calls.len() as i64 + 1;
    let mut all_output_messages = vec![];
    for msg in generated_tool.into_iter().chain(generated_other.into_iter()) {
        let dest_num = if let Some(dest_num) = tool_id_to_num.get(&msg.tool_call_id) {
            dest_num.clone()
        } else {
            afterwards_num += 1;
            afterwards_num - 1
        };
        let output_thread_messages = crate::cloud::messages_req::convert_messages_to_thread_messages(
            vec![msg], alt, alt, dest_num, &thread.ft_id, last_message.ftm_user_preferences.clone(),
        )?;
        all_output_messages.extend(output_thread_messages);
    }

    if !required_confirmation.is_empty() {
        if !crate::cloud::threads_req::set_thread_confirmation_request(
            cmd_address_url, api_key, &thread.ft_id, serde_json::to_value(required_confirmation.clone()).unwrap()
        ).await? {
            warn!("tool use: cannot set confirmation requests: {:?}", required_confirmation);
        }
    }

    if !all_output_messages.is_empty() {
        crate::cloud::messages_req::create_thread_messages(cmd_address_url, api_key, &thread.ft_id, all_output_messages).await?;
    } else {
        info!("thread `{}` has no tool output messages. Skipping it", thread.ft_id);
    }
    Ok(())
}

pub async fn process_thread_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: ThreadPayload,
    basic_info: BasicStuff,
    cmd_address_url: String,
    api_key: String,
    located_fgroup_id: String,
) -> Result<(), String> {
    if thread_payload.ft_need_tool_calls == -1
        || thread_payload.owner_fuser_id != basic_info.fuser_id
        || !thread_payload.ft_locked_by.is_empty() {
        return Ok(());
    }
    if let Some(error) = thread_payload.ft_error.as_ref() {
        info!("thread `{}` has the error: `{}`. Skipping it", thread_payload.ft_id, error);
        return Ok(());
    }

    let hash = generate_random_hash(16);
    let thread_id = thread_payload.ft_id.clone();
    let lock_result = lock_thread(&cmd_address_url, &api_key, &thread_id, &hash).await;
    if let Err(err) = lock_result {
        info!("failed to lock thread `{}` with hash `{}`: {}", thread_id, hash, err);
        return Ok(());
    }
    info!("thread `{}` locked successfully with hash `{}`", thread_id, hash);
    let process_result = process_locked_thread(
        gcx,
        &thread_payload,
        &thread_id,
        &cmd_address_url,
        &api_key,
        &located_fgroup_id
    ).await;
    match crate::cloud::threads_req::unlock_thread(&cmd_address_url, &api_key, &thread_id, &hash).await {
        Ok(_) => info!("thread `{}` unlocked successfully", thread_id),
        Err(err) => {
            error!("failed to unlock thread `{}`: {}", thread_id, err);
        },
    }
    process_result
}

pub async fn process_thread_message_event(
    gcx: Arc<ARwLock<GlobalContext>>,
    mut thread_message_payload: ThreadMessagePayload,
    basic_info: BasicStuff,
    cmd_address_url: String,
    api_key: String,
    located_fgroup_id: String,
) -> Result<(), String> {
    let old_message_cutoff = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() - 300;
    if thread_message_payload.ftm_role != "user" || thread_message_payload.ftm_created_ts < old_message_cutoff as f64 {
        return Ok(());
    }
    if thread_message_payload.ftm_app_specific.as_ref().is_some_and(|a| a.get("checkpoints").is_some()) {
        return Ok(());
    }
    let (checkpoints, _) = create_workspace_checkpoint(gcx.clone(), None, &thread_message_payload.ftm_belongs_to_ft_id).await?;

    let mut app_specific = thread_message_payload.ftm_app_specific.as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_else(|| serde_json::Map::new());
    app_specific.insert("checkpoints".to_string(), serde_json::json!(checkpoints));
    thread_message_payload.ftm_app_specific = Some(Value::Object(app_specific));

    tracing::info!("Created checkpoints: {:#?}", checkpoints);

    Ok(())
}

async fn process_locked_thread(
    gcx: Arc<ARwLock<GlobalContext>>,
    thread_payload: &ThreadPayload,
    thread_id: &str,
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str
) -> Result<(), String> {
    let alt = thread_payload.ft_need_tool_calls;
    let messages = match crate::cloud::messages_req::get_thread_messages(
        &cmd_address_url,
        &api_key,
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
    let thread = match crate::cloud::threads_req::get_thread(cmd_address_url, api_key, thread_id).await {
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
        initialize_thread(gcx.clone(), &ft_fexp_id, &thread, cmd_address_url, api_key, located_fgroup_id).await
    } else {
        info!("calling tools for thread `{}`", thread_id);
        call_tools(gcx.clone(), &thread, &messages, alt, cmd_address_url, api_key).await
    };
    if let Err(err) = &result {
        info!("failed to process thread `{}`, setting error: {}", thread_id, err);
        if let Err(set_err) = crate::cloud::threads_req::set_error_thread(
            cmd_address_url, api_key, thread_id, err
        ).await {
            return Err(format!("Failed to set error on thread: {}", set_err));
        }
    }
    result
}
