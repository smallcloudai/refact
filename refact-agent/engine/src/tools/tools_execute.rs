use std::collections::HashMap;
use std::sync::Arc;
use glob::Pattern;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::MIN_RAG_CONTEXT_LIMIT;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum, ContextFile, SubchatParameters};
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::scratchpads::scratchpad_utils::max_tokens_for_rag_chat_by_tools;
use crate::tools::tools_description::{MatchConfirmDenyResult, Tool};
use crate::yaml_configs::customization_loader::load_customization;


pub async fn unwrap_subchat_params(ccx: Arc<AMutex<AtCommandsContext>>, tool_name: &str) -> Result<SubchatParameters, String> {
    let (gcx, params_mb) = {
        let ccx_locked = ccx.lock().await;
        let gcx = ccx_locked.global_context.clone();
        let params = ccx_locked.subchat_tool_parameters.get(tool_name).cloned();  // comes from the request, the request has specified parameters
        (gcx, params)
    };
    let params = match params_mb {
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
    Ok(params)
}


pub async fn run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tools: &mut IndexMap<String, Box<dyn Tool+Send>>,
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
                let tool_failed_message = tool_answer_err(e, t_call.id.to_string());
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
        style,
    ).await;

    let new_messages = generated_tool.into_iter().chain(generated_other.into_iter())
        .collect::<Vec<_>>();

    ccx.lock().await.pp_skeleton = false;

    Ok((new_messages, true))
}

pub(crate) async fn pp_run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    original_messages: &Vec<ChatMessage>,
    any_corrections: bool,
    mut generated_tool: Vec<ChatMessage>,
    mut generated_other: Vec<ChatMessage>,
    context_files_for_pp: &mut Vec<ContextFile>,
    tokens_for_rag: usize,
    style: &Option<String>,
) -> (Vec<ChatMessage>, Vec<ChatMessage>) {
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
            generated_tool.into_iter().chain(generated_other.into_iter()).collect(), tokens_limit_chat_msg, style,
        ).await;

        // re-add potentially truncated messages, role="tool" will still go first
        generated_tool = Vec::new();
        generated_other = Vec::new();
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
            gcx.clone(), context_files_for_pp, tokens_limit_files, false, &pp_settings,
        ).await;

        if !context_file_vec.is_empty() {
            let json_vec: Vec<_> = context_file_vec.into_iter().map(|p| json!(p)).collect();
            let message = ChatMessage::new(
                "context_file".to_string(),
                serde_json::to_string(&json_vec).unwrap()
            );
            generated_other.push(message);
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


pub(crate) fn tool_answer_err(content: String, tool_call_id: String) -> ChatMessage {
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
