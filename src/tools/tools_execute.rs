use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::MIN_RAG_CONTEXT_LIMIT;
use crate::call_validation::{ChatMessage, ContextEnum, SubchatParameters};
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::scratchpads::scratchpad_utils::{HasRagResults, max_tokens_for_rag_chat};
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
) -> (Vec<ChatMessage>, bool)
{
    let (n_ctx, top_n, correction_only_up_to_step) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.n_ctx, ccx_locked.top_n, ccx_locked.correction_only_up_to_step)
    };
    let reserve_for_context = max_tokens_for_rag_chat(n_ctx, maxgen);
    let tokens_for_rag = reserve_for_context;
    {
        let mut ccx_locked = ccx.lock().await;
        ccx_locked.tokens_for_rag = tokens_for_rag;
    };

    info!("run_tools: reserve_for_context {} tokens", reserve_for_context);
    if original_messages.is_empty() {
        return (original_messages.clone(), false);
    }
    let ass_n = original_messages.len() - 1;
    let ass_msg = original_messages.get(ass_n).unwrap();

    if ass_msg.role != "assistant" {
        return (original_messages.clone(), false);
    }
    if ass_msg.tool_calls.is_none() || ass_msg.tool_calls.as_ref().unwrap().len() == 0 {
        return (original_messages.clone(), false);
    }

    let at_tools = ccx.lock().await.at_tools.clone();

    let mut for_postprocessing = vec![];
    let mut generated_tool = vec![];  // tool results must go first
    let mut generated_other = vec![];
    let mut any_corrections = false;

    for t_call in ass_msg.tool_calls.as_ref().unwrap_or(&vec![]).iter() {
        if let Some(cmd) = at_tools.get(&t_call.function.name) {
            info!("tool use: trying to run {:?}", &t_call.function.name);

            let args_maybe = serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments);
            if let Err(e) = args_maybe {
                let tool_failed_message = ChatMessage {
                    role: "tool".to_string(),
                    content: format!("couldn't deserialize arguments: {}. Error:\n{}\nTry again following JSON format", t_call.function.arguments, e),
                    tool_calls: None,
                    tool_call_id: t_call.id.to_string(),
                    ..Default::default()
                };
                generated_tool.push(tool_failed_message.clone());
                continue;
            }
            let args = args_maybe.unwrap();
            info!("tool use: args={:?}", args);
            let tool_msg_and_maybe_more_mb = cmd.lock().await.tool_execute(ccx.clone(), &t_call.id.to_string(), &args).await;
            if let Err(e) = tool_msg_and_maybe_more_mb {
                let mut tool_failed_message = ChatMessage {
                    role: "tool".to_string(),
                    content: e.to_string(),
                    tool_calls: None,
                    tool_call_id: t_call.id.to_string(),
                    ..Default::default()
                };
                {
                    let mut cmd_lock = cmd.lock().await;
                    if let Some(usage) = cmd_lock.usage() {
                        tool_failed_message.usage = Some(usage.clone());
                    }
                    *cmd_lock.usage() = None;
                }
                generated_tool.push(tool_failed_message.clone());
                continue;
            };
            let (corrections, tool_msg_and_maybe_more) = tool_msg_and_maybe_more_mb.unwrap();
            any_corrections |= corrections;
            let mut have_answer = false;
            for msg in tool_msg_and_maybe_more {
                if let ContextEnum::ChatMessage(ref raw_msg) = msg {
                    if (raw_msg.role == "tool" || raw_msg.role == "diff") && raw_msg.tool_call_id == t_call.id {
                        generated_tool.push(raw_msg.clone());
                        have_answer = true;
                    } else {
                        generated_other.push(raw_msg.clone());
                        assert!(raw_msg.tool_call_id.is_empty());
                    }
                }
                if let ContextEnum::ContextFile(ref cf) = msg {
                    for_postprocessing.push(cf.clone());
                }
            }
            assert!(have_answer);
        } else {
            let e = format!("tool use: function {:?} not found", &t_call.function.name);
            warn!(e);
            let tool_failed_message = ChatMessage {
                role: "tool".to_string(),
                content: e.to_string(),
                tool_calls: None,
                tool_call_id: t_call.id.to_string(),
                ..Default::default()
            };
            generated_tool.push(tool_failed_message.clone());
        }
    }

    if any_corrections && original_messages.len() <= correction_only_up_to_step {
        generated_other.clear();
        generated_other.push(ChatMessage::new("user".to_string(), format!("💿 There are corrections in the tool calls, all the output files are suppressed. Call again with corrections.")));

    } else if tokens_for_rag > MIN_RAG_CONTEXT_LIMIT {
        let (tokens_limit_chat_msg, mut tokens_limit_files) = {
            if for_postprocessing.is_empty() {
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
            pp_settings.take_floor = 9.0;
        }

        let context_file_vec = postprocess_context_files(
            gcx.clone(),
            &for_postprocessing,
            tokenizer.clone(),
            tokens_limit_files,
            false,
            &pp_settings,
        ).await;

        if !context_file_vec.is_empty() {
            let json_vec = context_file_vec.iter().map(|p| json!(p)).collect::<Vec<_>>();
            let message = ChatMessage::new(
                "context_file".to_string(),
                serde_json::to_string(&json_vec).unwrap_or("".to_string()),
            );
            generated_other.push(message.clone());
        }
    } else {
        tracing::warn!("There are tool results, but tokens_for_rag={tokens_for_rag} is very small, bad things will happen.")
    }

    let mut all_messages = original_messages.iter().map(|m| m.clone()).collect::<Vec<_>>();
    for msg in generated_tool.iter() {
        all_messages.push(msg.clone());
        stream_back_to_user.push_in_json(json!(msg));
    }
    for msg in generated_other.iter() {
        all_messages.push(msg.clone());
        stream_back_to_user.push_in_json(json!(msg));
    }

    ccx.lock().await.pp_skeleton = false;

    (all_messages, true)
}
