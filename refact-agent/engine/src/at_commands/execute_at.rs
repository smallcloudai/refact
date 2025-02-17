use std::sync::{Arc, RwLock};
use tokio::sync::Mutex as AMutex;
use regex::Regex;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tracing::{info, warn};

use crate::at_commands::at_commands::{AtCommandsContext, AtParam, filter_only_context_file_from_context_tool};
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::http::http_post_json;
use crate::http::routers::v1::at_commands::{CommandExecutePost, CommandExecuteResponse};
use crate::integrations::docker::docker_container_manager::docker_container_get_host_lsp_port_to_connect;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::postprocessing::pp_plain_text::postprocess_plain_text;
use crate::scratchpads::scratchpad_utils::{HasRagResults, max_tokens_for_rag_chat};


pub const MIN_RAG_CONTEXT_LIMIT: usize = 256;


pub async fn run_at_commands_locally(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) -> (Vec<ChatMessage>, usize, bool) {
    let (n_ctx, top_n, is_preview, gcx) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.n_ctx, ccx_locked.top_n, ccx_locked.is_preview, ccx_locked.global_context.clone())
    };
    if !is_preview {
        let preview_cache = gcx.read().await.at_commands_preview_cache.clone();
        preview_cache.lock().await.clear();
    }
    let reserve_for_context = max_tokens_for_rag_chat(n_ctx, maxgen);
    info!("reserve_for_context {} tokens", reserve_for_context);

    let any_context_produced = false;

    let mut user_msg_starts = original_messages.len();
    let mut messages_with_at: usize = 0;
    while user_msg_starts > 0 {
        let message = original_messages.get(user_msg_starts - 1).unwrap().clone();
        if message.role == "user" {
            user_msg_starts -= 1;
            if message.content.content_text_only().contains("@") {
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
        let mut msg = original_messages[msg_idx].clone();
        // todo: make multimodal messages support @commands
        if let ChatContent::Multimodal(_) = &msg.content {
            rebuilt_messages.push(msg.clone());
            stream_back_to_user.push_in_json(json!(msg));
            continue;
        }
        let mut content = msg.content.content_text_only();
        let content_n_tokens = msg.content.count_tokens(tokenizer.clone(), &None).unwrap_or(0) as usize;

        let mut context_limit = reserve_for_context / messages_with_at.max(1);
        context_limit = context_limit.saturating_sub(content_n_tokens);

        info!("msg {} user_posted {:?} which is {} tokens, that leaves {} tokens for context of this message", msg_idx, crate::nicer_logs::first_n_chars(&content, 50), content_n_tokens, context_limit);

        let mut messages_exec_output = vec![];
        if content.contains("@") {
            let (res, _) = execute_at_commands_in_query(ccx.clone(), &mut content).await;
            messages_exec_output.extend(res);
        }

        let mut plain_text_messages = vec![];
        for exec_result in messages_exec_output.iter() {
            // at commands exec() can produce role "user" "assistant" "diff" "plain_text"
            if let ContextEnum::ChatMessage(raw_msg) = exec_result {  // means not context_file
                if raw_msg.role != "plain_text" {
                    rebuilt_messages.push(raw_msg.clone());
                    stream_back_to_user.push_in_json(json!(raw_msg));
                } else {
                    plain_text_messages.push(raw_msg);
                }
            }
        }

        // TODO: reduce context_limit by tokens(messages_exec_output)

        if context_limit > MIN_RAG_CONTEXT_LIMIT {
            let mut context_file_pp = filter_only_context_file_from_context_tool(&messages_exec_output);
            let (tokens_limit_plain, mut tokens_limit_files) = {
                if context_file_pp.is_empty() {
                    (context_limit, 0)
                } else {
                    (context_limit / 2, context_limit / 2)
                }
            };
            info!("context_limit {} tokens_limit_plain {} tokens_limit_files: {}", context_limit, tokens_limit_plain, tokens_limit_files);

            let t0 = std::time::Instant::now();

            let (pp_plain_text, non_used_plain) = postprocess_plain_text(
                plain_text_messages,
                tokenizer.clone(),
                tokens_limit_plain,
                &None,
            ).await;
            for m in pp_plain_text {
                // OUTPUT: plain text after all custom messages
                rebuilt_messages.push(m.clone());
                stream_back_to_user.push_in_json(json!(m));
            }
            tokens_limit_files += non_used_plain;
            info!("tokens_limit_files {}", tokens_limit_files);
            let (gcx, mut pp_settings, pp_skeleton) = {
                let ccx_locked = ccx.lock().await;
                (ccx_locked.global_context.clone(), ccx_locked.postprocess_parameters.clone(), ccx_locked.pp_skeleton)
            };
            pp_settings.max_files_n = top_n;
            if pp_skeleton {
                pp_settings.take_floor = 50.0;
            }
            let post_processed = postprocess_context_files(
                gcx.clone(),
                &mut context_file_pp,
                tokenizer.clone(),
                tokens_limit_files,
                false,
                &pp_settings,
            ).await;
            if !post_processed.is_empty() {
                // OUTPUT: files after all custom messages and plain text
                let json_vec = post_processed.iter().map(|p| { json!(p)}).collect::<Vec<Value>>();
                if !json_vec.is_empty() {
                    let message = ChatMessage::new(
                        "context_file".to_string(),
                        serde_json::to_string(&json_vec).unwrap_or("".to_string()),
                    );
                    rebuilt_messages.push(message.clone());
                    stream_back_to_user.push_in_json(json!(message));
                }
            }
            info!("postprocess_plain_text_messages + postprocess_context_files {:.3}s", t0.elapsed().as_secs_f32());
        }

        if content.trim().len() > 0 {
            // stream back to the user, with at-commands replaced
            msg.content = ChatContent::SimpleText(content);
            rebuilt_messages.push(msg.clone());
            stream_back_to_user.push_in_json(json!(msg));
        }
    }

    ccx.lock().await.pp_skeleton = false;

    return (rebuilt_messages, user_msg_starts, any_context_produced)
}

pub async fn run_at_commands_remotely(
    ccx: Arc<AMutex<AtCommandsContext>>,
    model_name: &str,
    maxgen: usize,
    original_messages: &Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) -> Result<(Vec<ChatMessage>, usize, bool), String> {
    let (gcx, n_ctx, subchat_tool_parameters, postprocess_parameters, chat_id) = {
        let ccx_locked = ccx.lock().await;
        (
            ccx_locked.global_context.clone(),
            ccx_locked.n_ctx,
            ccx_locked.subchat_tool_parameters.clone(),
            ccx_locked.postprocess_parameters.clone(),
            ccx_locked.chat_id.clone()
        )
    };

    let post = CommandExecutePost {
        messages: original_messages.clone(),
        n_ctx,
        maxgen,
        subchat_tool_parameters,
        postprocess_parameters,
        model_name: model_name.to_string(),
        chat_id: chat_id.clone(),
    };

    let port = docker_container_get_host_lsp_port_to_connect(gcx.clone(), &chat_id).await?;
    tracing::info!("run_at_commands_remotely: connecting to port {}", port);

    let url = format!("http://localhost:{port}/v1/at-command-execute");
    let response: CommandExecuteResponse = http_post_json(&url, &post).await?;

    for msg in response.messages_to_stream_back {
        stream_back_to_user.push_in_json(msg);
    }

    Ok((response.messages, response.undroppable_msg_number, response.any_context_produced))
}

pub async fn correct_at_arg(
    ccx: Arc<AMutex<AtCommandsContext>>,
    param: Arc<AMutex<dyn AtParam>>,
    arg: &mut AtCommandMember,
) {
    let param_lock = param.lock().await;
    if param_lock.is_value_valid(ccx.clone(), &arg.text).await {
        return;
    }
    let completion = match param_lock.param_completion(ccx.clone(), &arg.text).await.get(0) {
        Some(x) => x.clone(),
        None => {
            arg.ok = false;
            arg.reason = Some("incorrect argument; failed to complete".to_string());
            return;
        }
    };
    if !param_lock.is_value_valid(ccx.clone(), &completion).await {
        arg.ok = false; arg.reason = Some("incorrect argument; completion did not help".to_string());
        return;
    }
    arg.text = completion;
}

pub async fn execute_at_commands_in_query(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &mut String,
) -> (Vec<ContextEnum>, Vec<AtCommandMember>) {
    let at_commands = {
        ccx.lock().await.at_commands.clone()
    };
    let at_command_names = at_commands.keys().map(|x|x.clone()).collect::<Vec<_>>();
    let mut context_enums = vec![];
    let mut highlight_members = vec![];
    let mut clips = vec![];

    let words = parse_words_from_line(query);
    for (w_idx, (word, pos1, pos2)) in words.iter().enumerate() {
        let cmd = match at_commands.get(word) {
            Some(c) => c.clone(),
            None => { continue; }
        };
        let cmd_lock = cmd.lock().await;
        let args = words.iter().skip(w_idx + 1).map(|x|x.clone()).collect::<Vec<_>>();

        let mut cmd_member = AtCommandMember::new("cmd".to_string(), word.clone(), *pos1, *pos2);
        let mut arg_members = vec![];
        for (text, pos1, pos2) in args.iter().map(|x|x.clone()) {
            if at_command_names.contains(&text) { break; }
            // TODO: break if there's \n\n
            arg_members.push(AtCommandMember::new("arg".to_string(), text.clone(), pos1, pos2));
        }

        match cmd_lock.at_execute(ccx.clone(), &mut cmd_member, &mut arg_members).await {
            Ok((res, text_on_clip)) => {
                context_enums.extend(res);
                clips.push((text_on_clip, cmd_member.pos1, arg_members.last().map(|x|x.pos2).unwrap_or(cmd_member.pos2)));
            },
            Err(e) => {
                cmd_member.ok = false; cmd_member.reason = Some(format!("incorrect argument; failed to complete: {}", e));
                warn!("can't execute command that indicated it can execute: {}", e);
            }
        }
        highlight_members.push(cmd_member);
        highlight_members.extend(arg_members);
    }
    for (text_on_clip, pos1, pos2) in clips.iter().rev() {
        // info!("replacing {:?}..{:?} with {:?}", *pos1, *pos2, text_on_clip);
        query.replace_range(*pos1..*pos2, text_on_clip);
    }
    // info!("query after at-commands: \n{:?}\n", query);
    (context_enums, highlight_members)
}

#[derive(Debug, Clone, Default)]
pub struct AtCommandMember {
    pub kind: String,
    pub text: String,
    pub pos1: usize,
    pub pos2: usize,
    pub ok: bool,
    pub reason: Option<String>,
}

impl AtCommandMember {
    pub fn new(kind: String, text: String, pos1: usize, pos2: usize) -> Self {
        Self { kind, text, pos1, pos2, ok: true, reason: None}
    }
}

pub fn parse_words_from_line(line: &String) -> Vec<(String, usize, usize)> {
    fn trim_punctuation(s: &str) -> String {
        s.trim_end_matches(&['!', '.', ',', '?'][..]).to_string()
    }

    // let word_regex = Regex::new(r#"(@?[^ !?@\n]*)"#).expect("Invalid regex");
    // let word_regex = Regex::new(r#"(@?[^ !?@\n]+|\n|@)"#).expect("Invalid regex");
    let word_regex = Regex::new(r#"(@?\S*)"#).expect("Invalid regex");         // fixed windows

    let mut results = vec![];
    for cap in word_regex.captures_iter(line) {
        if let Some(matched) = cap.get(1) {
            let trimmed_match = trim_punctuation(&matched.as_str().to_string());
            results.push((trimmed_match.clone(), matched.start(), matched.start() + trimmed_match.len()));
        }
    }
    results
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_words_from_line_with_link() {
        let line = "Check out this link: https://doc.rust-lang.org/book/ch03-04-comments.html".to_string();
        let parsed_words = parse_words_from_line(&line);

        let link = parsed_words.iter().find(|(word, _, _)| word == "https://doc.rust-lang.org/book/ch03-04-comments.html");
        assert!(link.is_some(), "The link should be parsed as a single word");
        if let Some((word, _start, _end)) = link {
            assert_eq!(word, "https://doc.rust-lang.org/book/ch03-04-comments.html");
        }
    }
}
