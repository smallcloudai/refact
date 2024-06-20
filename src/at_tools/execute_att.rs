use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::global_context::GlobalContext;
use crate::scratchpads::chat_utils_rag::{HasRagResults, max_tokens_for_rag_chat, postprocess_at_results2};


pub async fn run_tools(
    global_context: Arc<ARwLock<GlobalContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    n_ctx: usize,
    original_messages: &Vec<ChatMessage>,
    top_n: usize,
    stream_back_to_user: &mut HasRagResults,
) -> (Vec<ChatMessage>, bool)
{
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

    let mut ccx = AtCommandsContext::new(global_context.clone(), top_n, false, original_messages).await;
    let at_tools = ccx.at_tools.clone();

    let mut context_messages: Vec<ChatMessage> = original_messages.iter().map(|m| m.clone()).collect();
    let mut for_postprocessing: Vec<ContextFile> = vec![];

    for t_call in ass_msg.tool_calls.as_ref().unwrap_or(&vec![]).iter() {
        if let Some(cmd) = at_tools.get(&t_call.function.name) {
            info!("tool use: trying to run {:?}", &t_call.function.name);

            let args_maybe = serde_json::from_str::<HashMap<String, Value>>(&t_call.function.arguments);
            if let Err(e) = args_maybe {
                let tool_failed_message = ChatMessage {
                    role: "tool".to_string(),
                    content: e.to_string(),
                    tool_calls: None,
                    tool_call_id: t_call.id.to_string(),
                };
                context_messages.push(tool_failed_message.clone());
                stream_back_to_user.push_in_json(json!(tool_failed_message));
                continue;
            }
            let args = args_maybe.unwrap();
            info!("tool use: args={:?}", args);
            let tool_msg_and_maybe_more_mb = cmd.lock().await.execute(&mut ccx, &t_call.id.to_string(), &args).await;
            if let Err(e) = tool_msg_and_maybe_more_mb {
                let tool_failed_message = ChatMessage {
                    role: "tool".to_string(),
                    content: e.to_string(),
                    tool_calls: None,
                    tool_call_id: t_call.id.to_string(),
                };
                context_messages.push(tool_failed_message.clone());
                stream_back_to_user.push_in_json(json!(tool_failed_message));
                continue;
            }
            let tool_msg_and_maybe_more = tool_msg_and_maybe_more_mb.unwrap();
            let mut have_answer = false;
            for msg in tool_msg_and_maybe_more {
                if let ContextEnum::ChatMessage(ref raw_msg) = msg {
                    context_messages.push(raw_msg.clone());
                    stream_back_to_user.push_in_json(json!(raw_msg.clone()));
                    if (raw_msg.role == "tool" || raw_msg.role == "diff") && raw_msg.tool_call_id == t_call.id {
                        have_answer = true;
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
            };
            context_messages.push(tool_failed_message.clone());
            stream_back_to_user.push_in_json(json!(tool_failed_message));
        }
    }

    let context_file: Vec<ContextFile> = postprocess_at_results2(
        global_context.clone(),
        &for_postprocessing,
        tokenizer.clone(),
        context_limit,
        false,
        top_n,
    ).await;

    if !context_file.is_empty() {
        let json_vec = context_file.iter().map(|p| {
            json!(p)
        }).collect::<Vec<Value>>();

        let message = ChatMessage::new(
            "context_file".to_string(),
            serde_json::to_string(&json_vec).unwrap_or("".to_string()),
        );

        context_messages.push(message.clone());
        stream_back_to_user.push_in_json(json!(message));
    }

    (context_messages, true)
}
