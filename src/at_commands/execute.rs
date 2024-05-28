use std::sync::{Arc, RwLock};
use itertools::Itertools;
use regex::Regex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::{AtCommandCall, AtCommandsContext, filter_only_context_file_from_context_tool};
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};
use crate::global_context::GlobalContext;
use crate::scratchpads::chat_utils_rag::{count_tokens, HasRagResults, max_tokens_for_rag_chat, postprocess_at_results2};


pub async fn run_at_commands(
    global_context: Arc<ARwLock<GlobalContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    n_ctx: usize,
    original_messages: &Vec<ChatMessage>,
    top_n: usize,
    stream_back_to_user: &mut HasRagResults,
) -> (Vec<ChatMessage>, usize) {
    let reserve_for_context = max_tokens_for_rag_chat(n_ctx, maxgen);
    info!("reserve_for_context {} tokens", reserve_for_context);

    let mut ccx = AtCommandsContext::new(global_context.clone(), top_n).await;

    let mut user_msg_starts = original_messages.len();
    let mut messages_with_at: usize = 0;
    while user_msg_starts > 0 {
        let message = original_messages.get(user_msg_starts - 1).unwrap().clone();
        if message.role == "user" {
            user_msg_starts -= 1;
            if message.content.contains("@") {
                messages_with_at += 1;
            }
        } else {
            break;
        }
    }

    // Token limit works like this:
    // - if there's only 1 user message at the bottom, it receives reserve_for_context tokens for context
    // - if there are N user messages, they receive reserve_for_context/N tokens each (and there's no taking from one to give to the other)
    // This is useful to give prefix and suffix of the same file precisely the position necessary for FIM-like operation of a chat model
    let mut rebuilt_messages: Vec<ChatMessage> = original_messages.iter().take(user_msg_starts).map(|m| m.clone()).collect();
    for msg_idx in user_msg_starts..original_messages.len() {
        let msg = original_messages[msg_idx].clone();
        let role = msg.role.clone();

        let mut content = msg.content.clone();
        let content_n_tokens = count_tokens(&tokenizer.read().unwrap(), &content);
        let mut context_limit = reserve_for_context / messages_with_at.max(1);
        if context_limit <= content_n_tokens {
            context_limit = 0;
        } else {
            context_limit -= content_n_tokens;
        }
        info!("msg {} user_posted {:?} which is {} tokens, that leaves {} tokens for context of this message", msg_idx, crate::nicer_logs::first_n_chars(&content, 50), content_n_tokens,context_limit);

        let mut messages_exec_output = vec![];
        if content.contains("@") {
            let (res, _) = execute_at_commands_in_query(&mut ccx, &mut content, true).await;
            messages_exec_output.extend(res);
        }

        for exec_result in messages_exec_output.iter() {
            // at commands exec() can produce both role="user" and role="assistant" messages
            if let ContextEnum::ChatMessage(raw_msg) = exec_result {
                rebuilt_messages.push(raw_msg.clone());
                stream_back_to_user.push_in_json(json!(raw_msg));
            }
        }

        // TODO: reduce context_limit by tokens(messages_exec_output)
        let t0 = std::time::Instant::now();
        let post_processed: Vec<ContextFile> = postprocess_at_results2(
            global_context.clone(),
            &filter_only_context_file_from_context_tool(&messages_exec_output),
            tokenizer.clone(),
            context_limit,
            false,
        ).await;
        if post_processed.len() > 0 {
            // post-processed files after all custom messages
            let json_vec = post_processed.iter().map(|p| {
                json!(p)
            }).collect::<Vec<Value>>();
            if json_vec.len() > 0 {
                let message = ChatMessage::new(
                    "context_file".to_string(),
                    serde_json::to_string(&json_vec).unwrap_or("".to_string()),
                );
                rebuilt_messages.push(message.clone());
                stream_back_to_user.push_in_json(json!(message));
            }
        }
        info!("postprocess_at_results2 {:.3}s", t0.elapsed().as_secs_f32());

        if content.trim().len() > 0 {
            // stream back to the user, with at-commands replaced
            let msg = ChatMessage::new(role.clone(), content);
            rebuilt_messages.push(msg.clone());
            if role == "user" {
                stream_back_to_user.push_in_json(json!(msg));
            }
        }
    }
    return (rebuilt_messages.clone(), user_msg_starts)
}

async fn correct_call_if_needed(
    call: &mut AtCommandCall,
    highlights_local: &mut Vec<AtCommandHighlight>,
    ccx: &AtCommandsContext,
    command_names: &Vec<String>,
) {
    let params = call.command.lock().await.params().iter().cloned().collect::<Vec<_>>();
    if params.len() != call.args.len() {
        highlights_local.iter_mut().for_each(|h| {
            h.ok = false; h.reason = Some("incorrect number of arguments".to_string());
        });
        return;
    }

    let mut corrected = vec![];
    for ((param, arg), h) in params.iter().zip(call.args.iter()).zip(highlights_local.iter_mut()) {
        if command_names.contains(arg) {
            h.ok = false; h.reason = Some("incorrect argument; is a command name".to_string());
            return;
        }
        let param = param.lock().await;
        if param.is_value_valid(arg, ccx).await {
            corrected.push(arg.clone());
            continue;
        }
        let completion = match param.complete(arg, ccx).await.get(0) {
            Some(x) => x.clone(),
            None => {
                h.ok = false; h.reason = Some("incorrect argument; failed to complete".to_string());
                return;
            }
        };
        if !param.is_value_valid(&completion, ccx).await {
            h.ok = false; h.reason = Some("incorrect argument; completion did not help".to_string());
            return;
        }
        corrected.push(completion);
    }
    call.args = corrected;
}

async fn execute_at_commands_from_query_line(
    line_n: usize,
    line: &mut String,
    query: &String,
    ccx: &mut AtCommandsContext,
    remove_valid_from_query: bool,
    msgs: &mut Vec<ContextEnum>,
    highlights: &mut Vec<AtCommandHighlight>,
) -> bool {
    let pos1_start = {
        let mut lines = query.lines().collect::<Vec<_>>();
        lines.truncate(line_n);

        let mut pos1_start = 0;
        for line_before in  lines {
            pos1_start += line_before.len() + 1;
        }
        pos1_start
    };

    let at_command_names = ccx.at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    info!("at-commands running {:?}; commands available {:?}", query, at_command_names);
    let line_words = parse_words_from_line(line);
    let mut line_words_cloned = line_words.iter().map(|(x, _, _)|x.clone()).collect::<Vec<_>>();
    let mut another_pass_needed = false;

    for (w_idx, (word, pos1, pos2)) in line_words.iter().enumerate() {
        if let Some(cmd) = ccx.at_commands.get(word) {
            let mut call = AtCommandCall::new(cmd.clone(), vec![]);
            let mut highlights_local = vec![];

            let cmd_params_cnt = cmd.lock().await.params().len();
            let mut q_cmd_args = line_words.iter().skip(w_idx + 1).collect::<Vec<_>>();
            q_cmd_args.truncate(cmd_params_cnt);
            call.args = q_cmd_args.iter().map(|(text, _, _)|text.clone()).collect();

            highlights_local.push(AtCommandHighlight::new("cmd".to_string(), line_n, w_idx, pos1_start + *pos1, pos1_start + *pos2));

            for (i, (_, pos1, pos2)) in q_cmd_args.iter().enumerate().map(|(i, arg)| (i + 1, arg)) {
                highlights_local.push(AtCommandHighlight::new("arg".to_string(), line_n, w_idx + i, pos1_start + *pos1, pos1_start + *pos2));
            }

            correct_call_if_needed(&mut call, &mut highlights_local, ccx, &at_command_names).await;

            let mut executed = false;
            let mut text_on_clip = String::new();
            if highlights_local.iter().all(|x|x.ok) {
                match call.command.lock().await.execute(
                    ccx, 
                    query, 
                    &call.args,
                    &line_words.iter().skip(w_idx + 1).map(|(text, _, _)|text.clone()).collect::<Vec<_>>(),
                ).await {
                    Ok((m_res, m_text_on_clip)) =>
                        {
                            executed = true;
                            text_on_clip = m_text_on_clip;
                            msgs.extend(m_res)
                        },
                    Err(e) => {
                        warn!("can't execute command that indicated it can execute: {}", e);
                    }
                }
            }

            // not on preview
            if remove_valid_from_query {
                if executed {
                    let mut indices_to_remove = vec![];
                    for h in highlights_local.iter() {
                        indices_to_remove.push(h.word_n);
                    }
                    line_words_cloned.insert(*indices_to_remove.iter().max().unwrap_or(&0usize) + 1, text_on_clip);
                    for i in indices_to_remove.iter().rev() {
                        line_words_cloned.remove(*i);
                    }
                    *line = line_words_cloned.join(" ");
                    // need to extract second time because indexes were shifted
                    another_pass_needed = true;
                }
            }

            highlights.extend(highlights_local);

            if another_pass_needed {
                break;
            }
        }
    }
    another_pass_needed
}

pub async fn execute_at_commands_in_query(
    ccx: &mut AtCommandsContext,
    query: &mut String,
    remove_valid_from_query: bool,
) -> (Vec<ContextEnum>, Vec<AtCommandHighlight>) {
    // called from preview and chat
    let mut msgs = vec![];
    let mut highlights = vec![];
    let mut new_lines = vec![];

    for (idx, mut line) in query.lines().map(|x|x.to_string()).enumerate() {
        loop {
            let another_pass_needed = execute_at_commands_from_query_line(
                idx, &mut line, query, ccx, remove_valid_from_query, &mut msgs, &mut highlights
            ).await;

            if !another_pass_needed {
                new_lines.push(line);
                break;
            }
        }
    }
    *query = new_lines.join("\n");
    (msgs, highlights)
}


#[derive(Debug)]
pub struct AtCommandHighlight {
    pub kind: String,
    pub line_n: usize,
    pub word_n: usize,
    pub pos1: usize,
    pub pos2: usize,
    pub ok: bool,
    pub reason: Option<String>,
}

impl AtCommandHighlight {
    pub fn new(kind: String, line_n: usize, word_n: usize, pos1: usize, pos2: usize) -> Self {
        Self { kind, line_n, word_n, pos1, pos2, ok: true, reason: None}
    }
}

pub fn parse_words_from_line(line: &String) -> Vec<(String, usize, usize)> {
    // TODO: make regex better
    let word_regex = Regex::new(r#"(@?[^ !?@]*)"#).expect("Invalid regex");
    let mut results = vec![];
    for cap in word_regex.captures_iter(line) {
        if let Some(matched) = cap.get(1) {
            results.push((matched.as_str().to_string(), matched.start(), matched.end()));
        }
    }
    results
}
