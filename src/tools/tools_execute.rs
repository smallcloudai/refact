use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use glob::Pattern;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::MIN_RAG_CONTEXT_LIMIT;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile, SubchatParameters};
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::scratchpads::scratchpad_utils::{HasRagResults, max_tokens_for_rag_chat};
use crate::tools::tools_description::load_generic_tool_config;
use crate::yaml_configs::customization_loader::load_customization;
use crate::caps::get_model_record;


pub async fn unwrap_subchat_params(ccx: Arc<AMutex<AtCommandsContext>>, tool_name: &str) -> Result<SubchatParameters, String> {
    let (gcx, params_mb) = {
        let ccx_locked = ccx.lock().await;
        let gcx = ccx_locked.global_context.clone();
        let params = ccx_locked.subchat_tool_parameters.get(tool_name).cloned();
        (gcx, params)
    };
    let params = match params_mb {
        Some(params) => params,
        None => {
            let tconfig = load_customization(gcx.clone(), true).await?;
            tconfig.subchat_tool_parameters.get(tool_name).cloned()
                .ok_or_else(|| format!("subchat params for tool {} not found (checked in Post and in Customization)", tool_name))?
        }
    };
    let _ = get_model_record(gcx, &params.subchat_model).await?; // check if the model exists
    Ok(params)
}

pub async fn run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) -> (Vec<ChatMessage>, bool) {
    let (n_ctx, at_tools) = {
        let ccx_lock = ccx.lock().await;
        (ccx_lock.n_ctx, ccx_lock.at_tools.clone())
    };
    let reserve_for_context = max_tokens_for_rag_chat(n_ctx, maxgen);
    let tokens_for_rag = reserve_for_context;
    ccx.lock().await.tokens_for_rag = tokens_for_rag;
    info!("run_tools: reserve_for_context {} tokens", reserve_for_context);

    if tokens_for_rag < MIN_RAG_CONTEXT_LIMIT {
        warn!("There are tool results, but tokens_for_rag={tokens_for_rag} is very small, bad things will happen.");
        return (original_messages.clone(), false);
    }

    let last_msg_tool_calls = match original_messages.last().filter(|m|m.role=="assistant") {
        Some(m) => m.tool_calls.clone().unwrap_or(vec![]),
        None => return (original_messages.clone(), false),
    };
    if last_msg_tool_calls.is_empty() {
        return (original_messages.clone(), false);
    }

    let mut context_files_for_pp = vec![];
    let mut generated_tool = vec![];  // tool results must go first
    let mut generated_other = vec![];
    let mut any_corrections = false;
    let mut generic_tool_config = None;

    for t_call in last_msg_tool_calls {
        let cmd = match at_tools.get(&t_call.function.name) {
            Some(cmd) => cmd.clone(),
            None => {
                let tool_failed_message = tool_answer(
                    format!("tool use: function {:?} not found", &t_call.function.name), t_call.id.to_string()
                );
                warn!("{}", tool_failed_message.content);
                generated_tool.push(tool_failed_message.clone());
                continue;
            }
        };

        let args = match serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments) {
            Ok(args) => args,
            Err(e) => {
                let tool_failed_message = tool_answer(
                    format!("Tool use: couldn't parse arguments: {}. Error:\n{}", t_call.function.arguments, e), t_call.id.to_string()
                );
                generated_tool.push(tool_failed_message);
                continue;
            }
        };
        info!("tool use {}({:?})", &t_call.function.name, args);

        let command_to_match = match {
            let cmd_lock = cmd.lock().await;
            cmd_lock.command_to_match_against_confirm_deny(&args)
        } {
            Ok(command_to_match) => command_to_match,
            Err(e) => {
                let tool_failed_message = tool_answer(
                    format!("tool use: {}", e), t_call.id.to_string()
                );
                generated_tool.push(tool_failed_message);
                continue;
            }
        };

        if !command_to_match.is_empty() {
            let gcx = ccx.lock().await.global_context.clone();
            if generic_tool_config.is_none() {
                generic_tool_config = match load_generic_tool_config(gcx.clone()).await {
                    Ok(g) => Some(g),
                    Err(e) => {
                        let tool_failed_message = tool_answer(format!("tool use: {}", e), t_call.id.to_string());
                        generated_tool.push(tool_failed_message);
                        continue;
                    }
                };
            }

            if let Some(generic_tool_cfg) = &generic_tool_config {
                let (is_denied, reason) = command_should_be_denied(&command_to_match, &generic_tool_cfg.commands_deny, false);
                if is_denied {
                    let tool_failed_message = tool_answer(
                        format!("tool use: {}", reason), t_call.id.to_string()
                    );
                    generated_tool.push(tool_failed_message);
                    continue;
                }
            }
        }

        let (corrections, tool_execute_results) = {
            let mut cmd_lock = cmd.lock().await;
            match cmd_lock.tool_execute(ccx.clone(), &t_call.id.to_string(), &args).await {
                Ok(msg_and_maybe_more) => msg_and_maybe_more,
                Err(e) => {
                    info!("tool use {}({:?}) FAILED: {}", &t_call.function.name, &args, e);
                    let mut tool_failed_message = tool_answer(e, t_call.id.to_string());

                    tool_failed_message.usage = cmd_lock.usage().clone();
                    *cmd_lock.usage() = None;

                    generated_tool.push(tool_failed_message.clone());
                    continue;
                }
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

    let (generated_tool, generated_other) = pp_run_tools(
        ccx.clone(),
        original_messages,
        any_corrections,
        generated_tool,
        generated_other,
        &mut context_files_for_pp,
        tokens_for_rag,
        tokenizer.clone(),
    ).await;

    let mut all_messages = original_messages.to_vec();
    for msg in generated_tool.iter().chain(generated_other.iter()) {
        all_messages.push(msg.clone());
        stream_back_to_user.push_in_json(json!(msg));
    }

    ccx.lock().await.pp_skeleton = false;

    (all_messages, true)
}

async fn pp_run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    original_messages: &Vec<ChatMessage>,
    any_corrections: bool,
    generated_tool: Vec<ChatMessage>,
    generated_other: Vec<ChatMessage>,
    context_files_for_pp: &mut Vec<ContextFile>,
    tokens_for_rag: usize,
    tokenizer: Arc<RwLock<Tokenizer>>,
) -> (Vec<ChatMessage>, Vec<ChatMessage>) {
    let mut generated_tool = generated_tool.to_vec();
    let mut generated_other = generated_other.to_vec();

    let (top_n, correction_only_up_to_step) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.top_n, ccx_locked.correction_only_up_to_step)
    };

    if any_corrections && original_messages.len() <= correction_only_up_to_step {
        generated_other.clear();
        generated_other.push(ChatMessage::new("cd_instruction".to_string(), "💿 There are corrections in the tool calls, all the output files are suppressed. Call again with corrections.".to_string()));

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
            let json_vec = context_file_vec.iter().map(|p| json!(p)).collect::<Vec<_>>();
            let message = ChatMessage::new(
                "context_file".to_string(),
                serde_json::to_string(&json_vec).unwrap()
            );
            let mut found_exact_same_message = false;
            for original_msg in original_messages.iter().rev() {
                if original_msg.role == "user" {
                    break;
                }
                if original_msg.content == message.content {
                    found_exact_same_message = true;
                    break;
                }
            }
            if !found_exact_same_message {
                generated_other.push(message.clone());
            } else {
                generated_other.push(ChatMessage::new(
                    "cd_instruction".to_string(),
                    "💿 Whoops, you are running in circles. You already have those files. Answer the question with what you have. Answer in the language the user prefers. Follow the system prompt.".to_string(),
                ));
            }
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


fn tool_answer(content: String, tool_call_id: String) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content,
        tool_calls: None,
        tool_call_id,
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
        return (true, format!("Command {} needs confirmation due to rule {}", command, rule));
    }
    (false, "".to_string())
}

pub fn command_should_be_denied(
    command: &String,
    commands_deny_rules: &Vec<String>,
    detailed: bool,
) -> (bool, String) {
    if let Some(rule) = commands_deny_rules.iter().find(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches(&command)
    }) {
        let message = if detailed {
            format!("Command {} is denied due to rule {}", command, rule)
        } else {
            format!("Command {} is denied", command)
        };
        return (true, message);
    }

    (false, "".to_string())
}
