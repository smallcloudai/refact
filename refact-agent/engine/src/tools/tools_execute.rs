use std::collections::HashMap;
use std::sync::Arc;
use glob::Pattern;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::MIN_RAG_CONTEXT_LIMIT;
use crate::call_validation::{ChatContent, ChatMessage, ChatModelType, ChatUsage, ContextEnum, ContextFile, SubchatParameters};
use crate::custom_error::MapErrToString;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::http::http_post_json;
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::scratchpads::scratchpad_utils::{HasRagResults, max_tokens_for_rag_chat_by_tools};
use crate::tools::tools_description::{MatchConfirmDenyResult, Tool};
use crate::yaml_configs::customization_loader::load_customization;
use crate::caps::{is_cloud_model, resolve_chat_model, resolve_model};
use crate::http::routers::v1::at_tools::{ToolExecuteResponse, ToolsExecutePost};


pub async fn unwrap_subchat_params(ccx: Arc<AMutex<AtCommandsContext>>, tool_name: &str) -> Result<SubchatParameters, String> {
    let (gcx, params_mb) = {
        let ccx_locked = ccx.lock().await;
        let gcx = ccx_locked.global_context.clone();
        let params = ccx_locked.subchat_tool_parameters.get(tool_name).cloned();  // comes from the request, the request has specified parameters
        (gcx, params)
    };

    let mut params = match params_mb {
        Some(params) => params,
        None => {
            let mut error_log = Vec::new();
            let tconfig = load_customization(gcx.clone(), true, &mut error_log).await;
            for e in error_log.iter() {
                tracing::error!("{e}");
            }
            tconfig.subchat_tool_parameters.get(tool_name).cloned()
                .ok_or_else(|| format!("subchat params for tool {} not found (checked in Post and in Customization)", tool_name))?
        }
    };

    // check if the models exist otherwise use the external chat model
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err_to_string()?;

    if !params.subchat_model.is_empty() {
        match resolve_chat_model(caps.clone(), &params.subchat_model) {
            Ok(_) => return Ok(params),
            Err(e) => {
                tracing::warn!("Specified subchat_model {} is not available: {}", params.subchat_model, e);
            }
        }
    }

    let current_model = ccx.lock().await.current_model.clone();
    let model_to_resolve = match params.subchat_model_type {
        ChatModelType::Light => &caps.defaults.chat_light_model,
        ChatModelType::Default => &caps.defaults.chat_default_model,
        ChatModelType::Thinking => &caps.defaults.chat_thinking_model,
    };

    params.subchat_model = match resolve_model(&caps.chat_models, model_to_resolve) {
        Ok(model_rec) => {
            if !is_cloud_model(&current_model) && is_cloud_model(&model_rec.base.id)
                && params.subchat_model_type != ChatModelType::Light {
                current_model.to_string()
            } else {
                model_rec.base.id.clone()
            }
        },
        Err(e) => {
            tracing::warn!("{:?} model is not available: {}. Using {} model as a fallback.",
                params.subchat_model_type, e, current_model);
            current_model
        }
    };

    tracing::info!("using model for subchat: {}", params.subchat_model);
    Ok(params)
}

pub async fn run_tools_remotely(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_id: &str,
    maxgen: usize,
    original_messages: &[ChatMessage],
    stream_back_to_user: &mut HasRagResults,
    style: &Option<String>,
) -> Result<(Vec<ChatMessage>, bool), String> {
    let (n_ctx, subchat_tool_parameters, postprocess_parameters, gcx, chat_id) = {
        let ccx_locked = ccx.lock().await;
        (
            ccx_locked.n_ctx,
            ccx_locked.subchat_tool_parameters.clone(),
            ccx_locked.postprocess_parameters.clone(),
            ccx_locked.global_context.clone(),
            ccx_locked.chat_id.clone(),
        )
    };

    let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), &chat_id).await?;
    info!("run_tools_remotely: connecting to port {}", port);

    let tools_execute_post = ToolsExecutePost {
        messages: original_messages.to_vec(),
        n_ctx,
        maxgen,
        subchat_tool_parameters,
        postprocess_parameters,
        model_name: model_id.to_string(),
        chat_id,
        style: style.clone(),
    };

    let url = format!("http://localhost:{port}/v1/tools-execute");
    let response: ToolExecuteResponse = http_post_json(&url, &tools_execute_post).await?;
    info!("run_tools_remotely: got response: {:?}", response);

    let mut all_messages = tools_execute_post.messages;

    for msg in response.messages {
        stream_back_to_user.push_in_json(json!(&msg));
        all_messages.push(msg);
    }

    Ok((all_messages, response.tools_ran))
}

pub async fn run_tools_locally(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tools: &mut IndexMap<String, Box<dyn Tool + Send>>,
    tokenizer: Option<Arc<Tokenizer>>,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
    style: &Option<String>,
) -> Result<(Vec<ChatMessage>, bool), String> {
    let (new_messages, tools_ran) = run_tools(
        ccx, tools, tokenizer, maxgen, original_messages, style
    ).await?;

    let mut all_messages = original_messages.to_vec();
    for msg in new_messages {
        stream_back_to_user.push_in_json(json!(&msg));
        all_messages.push(msg);
    }

    Ok((all_messages, tools_ran))
}

pub async fn run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tools: &mut IndexMap<String, Box<dyn Tool+Send>>,
    tokenizer: Option<Arc<Tokenizer>>,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    style: &Option<String>,
) -> Result<(Vec<ChatMessage>, bool), String> {
    let n_ctx = ccx.lock().await.n_ctx;
    // Default tokens limit for tools that perform internal compression (`tree()`, ...)
    ccx.lock().await.tokens_for_rag = 4096;

    let last_msg_tool_calls = match original_messages.last().filter(|m|m.role=="assistant") {
        Some(m) => m.tool_calls.clone().unwrap_or(vec![]),
        None => return Ok((vec![], false)),
    };
    if last_msg_tool_calls.is_empty() {
        return Ok((vec![], false));
    }

    let mut context_files_for_pp = vec![];
    let mut generated_tool = vec![];  // tool results must go first
    let mut generated_other = vec![];
    let mut any_corrections = false;

    for t_call in last_msg_tool_calls.iter() {
        let cmd = match tools.get_mut(&t_call.function.name) {
            Some(cmd) => cmd,
            None => {
                let tool_failed_message = tool_answer_err(
                    format!("tool use: function {:?} not found", &t_call.function.name), t_call.id.to_string()
                );
                warn!("{}", tool_failed_message.content.content_text_only());
                generated_tool.push(tool_failed_message.clone());
                continue;
            }
        };

        let args = match serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments) {
            Ok(args) => args,
            Err(e) => {
                let tool_failed_message = tool_answer_err(
                    format!("Tool use: couldn't parse arguments: {}. Error:\n{}", t_call.function.arguments, e), t_call.id.to_string()
                );
                generated_tool.push(tool_failed_message);
                continue;
            }
        };
        info!("tool use {}({:?})", &t_call.function.name, args);

        match cmd.match_against_confirm_deny(ccx.clone(), &args).await {
            Ok(res) => {
                match res.result {
                    MatchConfirmDenyResult::DENY => {
                        let command_to_match = cmd
                            .command_to_match_against_confirm_deny(ccx.clone(), &args).await
                            .unwrap_or("<error_command>".to_string());
                        generated_tool.push(tool_answer_err(format!("tool use: command '{command_to_match}' is denied"), t_call.id.to_string()));
                        continue;
                    }
                    _ => {}
                }
            }
            Err(err) => {
                generated_tool.push(tool_answer_err(format!("tool use: {}", err), t_call.id.to_string()));
                continue;
            }
        };

        let (corrections, tool_execute_results) = match cmd.tool_execute(ccx.clone(), &t_call.id.to_string(), &args).await {
            Ok((corrections, mut tool_execute_results)) => {
                for tool_execute_result in &mut tool_execute_results {
                    if let ContextEnum::ChatMessage(m) = tool_execute_result {
                        m.tool_failed = Some(false);
                    }
                }
                (corrections, tool_execute_results)
            }
            Err(e) => {
                warn!("tool use {}({:?}) FAILED: {}", &t_call.function.name, &args, e);
                let mut tool_failed_message = tool_answer_err(e, t_call.id.to_string());
                tool_failed_message.usage = cmd.usage().clone();
                *cmd.usage() = None;
                generated_tool.push(tool_failed_message.clone());
                continue;
            }
        };

        any_corrections |= corrections;

        let mut have_answer = false;
        for msg in tool_execute_results {
            match msg {
                ContextEnum::ChatMessage(m) => {
                    if (m.role == "tool" || m.role == "diff") && m.tool_call_id == t_call.id {
                        generated_tool.push(m);
                        have_answer = true;
                    } else {
                        assert!(m.tool_call_id.is_empty());
                        generated_other.push(m);
                    }
                },
                ContextEnum::ContextFile(m) => {
                    context_files_for_pp.push(m);
                }
            }
        }
        assert!(have_answer);
    }

    let reserve_for_context = max_tokens_for_rag_chat_by_tools(
        &last_msg_tool_calls,
        &context_files_for_pp,
        n_ctx, maxgen
    );
    let tokens_for_rag = reserve_for_context;
    ccx.lock().await.tokens_for_rag = tokens_for_rag;
    info!("run_tools: reserve_for_context {} tokens", reserve_for_context);
    if tokens_for_rag < MIN_RAG_CONTEXT_LIMIT {
        warn!("There are tool results, but tokens_for_rag={tokens_for_rag} is very small, bad things will happen.");
        return Ok((vec![], false));
    }

    let (generated_tool, generated_other) = pp_run_tools(
        ccx.clone(),
        original_messages,
        any_corrections,
        generated_tool,
        generated_other,
        &mut context_files_for_pp,
        tokens_for_rag,
        tokenizer.clone(),
        style,
    ).await;

    let new_messages = generated_tool.into_iter().chain(generated_other.into_iter())
        .collect::<Vec<_>>();

    ccx.lock().await.pp_skeleton = false;

    Ok((new_messages, true))
}

/// Detects duplicate tool calls in the conversation history and returns a CD instruction if found
fn detect_duplicate_tool_calls(
    original_messages: &Vec<ChatMessage>,
    current_tool_calls: &Vec<ChatToolCall>,
) -> Option<ChatMessage> {
    let mut seen_tool_calls = Vec::new();
    info!("DETECT_DUPLICATE_TOOL_CALLS::START");
    // Collect all tool calls from conversation history, excluding the last message (current)
    for msg in original_messages.iter().rev().skip(1).take(10) { // Skip current message, look at last 10
        if msg.role == "assistant" {
            if let Some(tool_calls) = &msg.tool_calls {
                for tool_call in tool_calls {
                    seen_tool_calls.push((&tool_call.function.name, &tool_call.function.arguments));
                    info!("DETECT_DUPLICATE_TOOL_CALLS::SEEN: {}({})", tool_call.function.name, 
                        if tool_call.function.arguments.len() > 50 { format!("{}...", &tool_call.function.arguments[..50]) } 
                        else { tool_call.function.arguments.clone() });
                }
            }
        }
    }
    
    // Check if any current tool call is a duplicate
    info!("DETECT_DUPLICATE_TOOL_CALLS::CURRENT_CALLS: {:?}", current_tool_calls.clone().into_iter().map(|c| c.function.name).collect::<Vec<String>>());
    for current_call in current_tool_calls {
        let current_name = &current_call.function.name;
        let current_signature = (&current_call.function.name, &current_call.function.arguments);
        let duplicate_count = seen_tool_calls.iter().filter(|&&call| call == current_signature).count();
        let tools_duplicate_not_allowed = vec!["search_symbol_definition", "search_symbol_usages", "tree", "create_textdoc", "update_textdoc", "update_textdoc_regex", "cat", "search_pattern", "search_semantic"];
        if duplicate_count >= 1 && tools_duplicate_not_allowed.contains(&current_name.as_str()) { // Found at least one previous identical call
            info!("DETECT_DUPLICATE_TOOL_CALLS::FOUND_DUPLICATE: {:?}", current_signature); 
            let cd_instruction = format!(
                "💿 STOP: Duplicate tool call detected! {}({}) has been called {} times with identical arguments. DO NOT call this same tool with the same arguments again. You must either: 1) Use different arguments, 2) Try a completely different tool or approach.",
                current_call.function.name,
                // Truncate arguments for readability  
                if current_call.function.arguments.len() > 100 {
                    format!("{}...", &current_call.function.arguments[..100])
                } else {
                    current_call.function.arguments.clone()
                },
                duplicate_count + 1
            );
            
            info!("DETECT_DUPLICATE_TOOL_CALLS::RETURNING_CD_INSTRUCTION: {}", cd_instruction);
            return Some(ChatMessage::new("cd_instruction".to_string(), cd_instruction));
        }
    }
    
    info!("DETECT_DUPLICATE_TOOL_CALLS::NO_DUPLICATES_FOUND");
    None
}

async fn pp_run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    original_messages: &Vec<ChatMessage>,
    any_corrections: bool,
    generated_tool: Vec<ChatMessage>,
    generated_other: Vec<ChatMessage>,
    context_files_for_pp: &mut Vec<ContextFile>,
    tokens_for_rag: usize,
    tokenizer: Option<Arc<Tokenizer>>,
    style: &Option<String>,
) -> (Vec<ChatMessage>, Vec<ChatMessage>) {
    let mut generated_tool = generated_tool.to_vec();
    let mut generated_other = generated_other.to_vec();

    let (top_n, correction_only_up_to_step) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.top_n, ccx_locked.correction_only_up_to_step)
    };

    // Check for duplicate tool calls and add CD instruction if found
    let mut duplicate_cd_instruction = None;
    if let Some(last_msg) = original_messages.last().filter(|m| m.role == "assistant") {
        if let Some(tool_calls) = &last_msg.tool_calls {
            duplicate_cd_instruction = detect_duplicate_tool_calls(original_messages, tool_calls);
        }
    }

    if any_corrections && original_messages.len() <= correction_only_up_to_step {
        generated_other.clear();
        generated_other.push(ChatMessage::new("cd_instruction".to_string(), "💿 There are corrections in the tool calls, all the output files are suppressed. Call again with corrections.".to_string()));
        
        // Re-add the duplicate detection CD instruction if it exists
        if let Some(duplicate_cd) = duplicate_cd_instruction {
            generated_other.push(duplicate_cd);
        }
    } else if tokens_for_rag > MIN_RAG_CONTEXT_LIMIT {
        let (tokens_limit_chat_msg, mut tokens_limit_files) = {
            if context_files_for_pp.is_empty() {
                (tokens_for_rag, 0)
            } else {
                (tokens_for_rag / 2, tokens_for_rag / 2)
            }
        };
        info!("run_tools: tokens_for_rag={} tokens_limit_chat_msg={} tokens_limit_files={}", tokens_for_rag, tokens_limit_chat_msg, tokens_limit_files);

        let (pp_chat_msg, non_used_tokens_for_rag) = postprocess_plain_text(
            generated_tool.iter().chain(generated_other.iter()).collect(),
            tokenizer.clone(),
            tokens_limit_chat_msg,
            style,
        ).await;

        // re-add potentially truncated messages, role="tool" will still go first
        generated_tool.clear();
        generated_other.clear();
        for m in pp_chat_msg {
            if !m.tool_call_id.is_empty() {
                generated_tool.push(m.clone());
            } else {
                generated_other.push(m.clone());
            }
        }

        tokens_limit_files += non_used_tokens_for_rag;
        info!("run_tools: tokens_limit_files={} after postprocessing", tokens_limit_files);

        let (gcx, mut pp_settings, pp_skeleton) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.postprocess_parameters.clone(), ccx_locked.pp_skeleton)
        };
        pp_settings.close_small_gaps = true;
        if pp_settings.max_files_n == 0 {
            pp_settings.max_files_n = top_n;
        }
        if pp_skeleton && pp_settings.take_floor == 0.0 {
            pp_settings.take_floor = 50.0;
        }

        let context_file_vec = postprocess_context_files(
            gcx.clone(),
            context_files_for_pp,
            tokenizer.clone(),
            tokens_limit_files,
            false,
            &pp_settings,
        ).await;

        if !context_file_vec.is_empty() {
            let json_vec: Vec<_> = context_file_vec.iter().map(|p| json!(p)).collect();
            let message = ChatMessage::new(
                "context_file".to_string(),
                serde_json::to_string(&json_vec).unwrap()
            );
            generated_other.push(message);
        }

        // Add duplicate detection CD instruction in normal flow too
        if let Some(duplicate_cd) = duplicate_cd_instruction {
            generated_other.push(duplicate_cd);
        }

    } else {
        warn!("There are tool results, but tokens_for_rag={tokens_for_rag} is very small, bad things will happen.")
    }

    // Sort generated_other such that cd_instruction comes last using stable sort
    generated_other.sort_by(|a, b| match (a.role.as_str(), b.role.as_str()) {
        ("cd_instruction", "cd_instruction") => std::cmp::Ordering::Equal,
        ("cd_instruction", _) => std::cmp::Ordering::Greater,
        (_, "cd_instruction") => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    });

    (generated_tool, generated_other)
}


fn tool_answer_err(content: String, tool_call_id: String) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: ChatContent::SimpleText(content),
        tool_calls: None,
        tool_call_id,
        tool_failed: Some(true),
        ..Default::default()
    }
}

pub fn command_should_be_confirmed_by_user(
    command: &String,
    commands_need_confirmation_rules: &Vec<String>,
) -> (bool, String) {
    if let Some(rule) = commands_need_confirmation_rules.iter().find(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches(&command)
    }) {
        return (true, rule.clone());
    }
    (false, "".to_string())
}

pub fn command_should_be_denied(
    command: &String,
    commands_deny_rules: &Vec<String>,
) -> (bool, String) {
    if let Some(rule) = commands_deny_rules.iter().find(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches(&command)
    }) {
        return (true, rule.clone());
    }

    (false, "".to_string())
}

pub fn update_usage_from_message(usage: &mut ChatUsage, message: &ChatMessage) {
    if let Some(u) = message.usage.as_ref() {
        usage.total_tokens += u.total_tokens;
        usage.completion_tokens += u.completion_tokens;
        usage.prompt_tokens += u.prompt_tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::ChatToolFunction;

    #[test]
    fn test_detect_duplicate_tool_calls() {
        // Create test messages with duplicate tool calls
        let mut messages = Vec::new();
        
        // First message with a tool call
        let tool_call_1 = ChatToolCall {
            id: "call_1".to_string(),
            function: ChatToolFunction {
                name: "cat".to_string(),
                arguments: r#"{"path": "test.txt"}"#.to_string(),
            },
            tool_type: "function".to_string(),
        };
        
        let msg_1 = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("I'll read the file".to_string()),
            tool_calls: Some(vec![tool_call_1.clone()]),
            ..Default::default()
        };
        messages.push(msg_1);
        
        // Tool response
        let tool_response = ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText("File content here".to_string()),
            tool_call_id: "call_1".to_string(),
            ..Default::default()
        };
        messages.push(tool_response);
        
        // Current message with duplicate tool call
        let current_tool_calls = vec![tool_call_1.clone()];
        
        // Test duplicate detection
        let result = detect_duplicate_tool_calls(&messages, &current_tool_calls);
        assert!(result.is_some());
        
        let cd_instruction = result.unwrap();
        assert_eq!(cd_instruction.role, "cd_instruction");
        assert!(cd_instruction.content.content_text_only().contains("💿 STOP: Duplicate tool call detected!"));
        assert!(cd_instruction.content.content_text_only().contains("cat"));
        assert!(cd_instruction.content.content_text_only().contains("Try a completely different tool"));
    }

    #[test]
    fn test_no_duplicate_tool_calls() {
        let mut messages = Vec::new();
        
        // First message with a tool call
        let tool_call_1 = ChatToolCall {
            id: "call_1".to_string(),
            function: ChatToolFunction {
                name: "cat".to_string(),
                arguments: r#"{"path": "test1.txt"}"#.to_string(),
            },
            tool_type: "function".to_string(),
        };
        
        let msg_1 = ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText("I'll read the first file".to_string()),
            tool_calls: Some(vec![tool_call_1]),
            ..Default::default()
        };
        messages.push(msg_1);
        
        // Current message with different tool call
        let current_tool_calls = vec![ChatToolCall {
            id: "call_2".to_string(),
            function: ChatToolFunction {
                name: "cat".to_string(),
                arguments: r#"{"path": "test2.txt"}"#.to_string(), // Different arguments
            },
            tool_type: "function".to_string(),
        }];
        
        // Test no duplicate detection
        let result = detect_duplicate_tool_calls(&messages, &current_tool_calls);
        assert!(result.is_none());
    }
}
