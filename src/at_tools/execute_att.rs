use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex as AMutex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::execute_at::MIN_RAG_CONTEXT_LIMIT;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::scratchpads::chat_utils_rag::{HasRagResults, max_tokens_for_rag_chat, postprocess_at_results2, postprocess_plain_text_messages};


pub async fn run_tools(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) -> (Vec<ChatMessage>, bool)
{
    let (n_ctx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.n_ctx, ccx_locked.top_n)
    };
    let reserve_for_context = max_tokens_for_rag_chat(n_ctx, maxgen);
    let context_limit = reserve_for_context;

    info!("run_tools: reserve_for_context {} tokens", reserve_for_context);
    if original_messages.len() == 0 {
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
    let mut generated_tool = vec![];  // tool must go first
    let mut generated_other = vec![];

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
            }
            let tool_msg_and_maybe_more = tool_msg_and_maybe_more_mb.unwrap();
            let mut have_answer = false;
            for msg in tool_msg_and_maybe_more {
                if let ContextEnum::ChatMessage(ref raw_msg) = msg {
                    if (raw_msg.role == "tool" || raw_msg.role == "diff" || raw_msg.role == "supercat") && raw_msg.tool_call_id == t_call.id {
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

    if context_limit > MIN_RAG_CONTEXT_LIMIT {
        let (tokens_limit_chat_msg, mut tokens_limit_files) = {
            if for_postprocessing.is_empty() {
                (context_limit, 0)
            } else {
                (context_limit / 2, context_limit / 2)
            }
        };
        info!("run_tools: context_limit={} tokens_limit_chat_msg={} tokens_limit_files={}", context_limit, tokens_limit_chat_msg, tokens_limit_files);

        let (pp_chat_msg, non_used_context_limit) = postprocess_plain_text_messages(
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

        tokens_limit_files += non_used_context_limit;
        info!("run_tools: tokens_limit_files={} after postprocessing", tokens_limit_files);

        let gcx = ccx.lock().await.global_context.clone();
        let (context_file_vec, _) = postprocess_at_results2(
            gcx.clone(),
            &for_postprocessing,
            tokenizer.clone(),
            tokens_limit_files,
            false,
            top_n,
        ).await;

        if !context_file_vec.is_empty() {
            let json_vec = context_file_vec.iter().map(|p| json!(p)).collect::<Vec<_>>();
            let message = ChatMessage::new(
                "context_file".to_string(),
                serde_json::to_string(&json_vec).unwrap_or("".to_string()),
            );
            generated_other.push(message.clone());
        }
    }

    let mut all_messages: Vec<ChatMessage> = original_messages.iter().map(|m| m.clone()).collect();
    for msg in generated_tool.iter() {
        all_messages.push(msg.clone());
        stream_back_to_user.push_in_json(json!(msg));
    }
    for msg in generated_other.iter() {
        all_messages.push(msg.clone());
        stream_back_to_user.push_in_json(json!(msg));
    }

    (all_messages, true)
}
