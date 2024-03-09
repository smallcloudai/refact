use std::sync::Arc;
use std::sync::RwLock;
use std::cmp::Ordering;
use tracing::info;
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_commands::AtCommandsContext;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile};
use crate::global_context::GlobalContext;


const RESERVE_FOR_QUESTION_AND_FOLLOWUP: usize = 1024;  // tokens
const SMALL_GAP_LINES: usize = 10;  // lines


pub fn count_tokens(
    tokenizer: &Tokenizer,
    text: &str,
) -> usize {
    match tokenizer.encode(text, false) {
        Ok(tokens) => tokens.len(),
        Err(_) => 0,
    }
}

pub async fn postprocess_at_results(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: Vec<ChatMessage>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
) -> Vec<ContextFile> {
    // 1. Decode all
    let mut cxfile_list: Vec<ContextFile> = vec![];
    for msg in messages {
        cxfile_list.extend(serde_json::from_str::<Vec<ContextFile>>(&msg.content).unwrap()); // TODO: this unwrap() is not good
    }
    // This check_only==true is for debugging only, can be safely removed (the result is already ignored)
    let _ = reload_files(global_context.clone(), &cxfile_list, true).await;
    // 2. Sort by usefulness
    cxfile_list.sort_by(|a, b| {
        b.usefulness.partial_cmp(&a.usefulness).unwrap_or(Ordering::Equal)
    });
    for cxfile in cxfile_list.iter() {
        info!("sorted file {}:{}-{} usefulness {:.1}", crate::nicer_logs::last_n_chars(&cxfile.file_name, 30), cxfile.line1, cxfile.line2, cxfile.usefulness);
    }
    // 3. Truncate less useful to tokens_limit
    let mut total_tokens: usize = 0;
    let mut idx = 0;
    while idx < cxfile_list.len() {
        let x: ContextFile = cxfile_list[idx].clone();
        let tokens_count = if total_tokens < tokens_limit { count_tokens(&tokenizer.read().unwrap(), x.file_content.as_str()) } else { 0 };
        total_tokens += tokens_count;
        if total_tokens > tokens_limit {
            cxfile_list.remove(idx);
            info!("drop less useful {}:{}-{} because {} tokens greater than limit {}", crate::nicer_logs::last_n_chars(&x.file_name, 30), x.line1, x.line2, total_tokens, tokens_limit);
        } else {
            info!("take {}:{}-{} tokens {} < {}", crate::nicer_logs::last_n_chars(&x.file_name, 30), x.line1, x.line2, total_tokens, tokens_limit);
            idx += 1;
        }
    }
    // 4. Remove small gaps in lines and deduplicate
    let mut merged: Vec<ContextFile> = vec![];
    let list_len = cxfile_list.len();
    let mut eaten: Vec<bool> = vec![false; list_len];
    loop {
        let mut merged_anything = false;
        let cxfile_list_copy = cxfile_list.clone();  // unnecessary operation because of rust borrow rules :/
        for i in 0..list_len {
            if eaten[i] {
                continue;
            }
            let x: &mut ContextFile = cxfile_list.get_mut(i).unwrap();
            for j in (i+1)..list_len {
                if eaten[j] {
                    continue;
                }
                let y: &ContextFile = cxfile_list_copy.get(j).unwrap();
                if x.file_name != y.file_name {
                    continue;
                }
                let possible_merge_line1 = x.line1.min(y.line1);
                let possible_merge_line2 = x.line2.max(y.line2);
                if possible_merge_line2 - possible_merge_line1 <= (x.line2 - x.line1) + (y.line2 - y.line1) + SMALL_GAP_LINES {
                    // good, makes sense to merge
                    info!("merging file {} range {}-{} with range {}-{}", crate::nicer_logs::last_n_chars(&x.file_name, 30), x.line1, x.line2, y.line1, y.line2);
                    eaten[j] = true;
                    x.line1 = possible_merge_line1;
                    x.line2 = possible_merge_line2;
                    x.usefulness = x.usefulness.max(y.usefulness);
                    // file_content is broken here and needs to be reloaded, see reload_files()
                    merged_anything = true;
                }
            }
        }
        if !merged_anything {
            break;
        }
    }
    for i in 0..list_len {
        if eaten[i] {
            continue;
        }
        merged.push(cxfile_list[i].clone());
        info!("merged {}:{}-{}", crate::nicer_logs::last_n_chars(&cxfile_list[i].file_name, 30), cxfile_list[i].line1, cxfile_list[i].line2);
    }
    merged
}

pub async fn reload_files(
    global_context: Arc<ARwLock<GlobalContext>>,
    merged: &Vec<ContextFile>,
    check_only: bool,
) -> Vec<ChatMessage>
{
    // drop old text in file_content, load new using get_file_text_from_memory_or_disk
    let mut was_able_to_reload: Vec<ContextFile> = vec![];
    for m in merged.iter() {
        let file_path = m.file_name.clone();
        let file_text_maybe: Result<String, String> = crate::files_in_workspace::get_file_text_from_memory_or_disk(global_context.clone(), &file_path).await;
        if file_text_maybe.is_err() {
            info!("file {} not found", file_path);
            continue;
        }
        let file_text = file_text_maybe.unwrap();
        if m.line1 == 0 || m.line2 == 0 {
            info!("file {} has invalid line range {}-{}", file_path, m.line1, m.line2);
            continue;
        }
        let line1: usize = m.line1 as usize;
        let line2: usize = m.line2 as usize;
        let content_line1_line2 = file_text.lines().skip(line1 - 1).take(line2 - line1 + 1).collect::<Vec<&str>>();
        // for s in content_line1_line2.clone() {
        //     info!("reloading {}", s);
        // }
        let content_line1_line2_str = content_line1_line2.join("\n") + "\n";
        if check_only {
            if m.file_content != content_line1_line2_str && m.file_content.clone() + "\n" != content_line1_line2_str {
                // this is mostly important because tokens limit might not work correctly (or a logical bug is a bad thing, too)
                info!("content of {}:{}-{} doesn't match file_content, bug or maybe the file has changed?", file_path, m.line1, m.line2);
                info!("file  : {:?}", m.file_content);
                info!("loaded: {:?}", content_line1_line2_str);
            }
            continue;
        }
        was_able_to_reload.push(ContextFile {
            file_name: m.file_name.clone(),
            file_content: content_line1_line2_str,
            line1: m.line1,
            line2: m.line2,
            usefulness: m.usefulness,
        });
    }

    // Encode back into a single message
    let mut processed_messages: Vec<ChatMessage> = vec![];
    if merged.len() == 0 {
        return processed_messages;
    }
    let message = ChatMessage {
        role: "context_file".to_string(),
        content: serde_json::to_string(&was_able_to_reload).unwrap(),
    };
    processed_messages.push(message);
    processed_messages
}

pub async fn run_at_commands(
    global_context: Arc<ARwLock<GlobalContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    n_ctx: usize,
    post: &mut ChatPost,
    top_n: usize,
    stream_back_to_user: &mut HasVecdbResults,
) -> usize {
    // TODO: don't operate on `post`, return a copy of the messages
    let context = AtCommandsContext::new(global_context.clone()).await;

    let mut user_msg_starts = post.messages.len();
    let mut user_messages_with_at: usize = 0;
    while user_msg_starts > 0 {
        let message = post.messages.get(user_msg_starts - 1).unwrap().clone();
        let role = message.role.clone();
        let content = message.content.clone();
        info!("user_msg_starts {} {}", user_msg_starts - 1, role);
        if role == "user" {
            user_msg_starts -= 1;
            if content.contains("@") {
                user_messages_with_at += 1;
            }
        } else {
            break;
        }
    }
    user_messages_with_at = user_messages_with_at.max(1);
    let reserve_for_context = n_ctx - maxgen - RESERVE_FOR_QUESTION_AND_FOLLOWUP;
    info!("reserve_for_context {} tokens", reserve_for_context);

    // Token limit works like this:
    // - if there's only 1 user message at the bottom, it receives ntokens_minus_maxgen tokens for context
    // - if there are N user messages, they receive ntokens_minus_maxgen/N tokens each (and there's no taking from one to give to the other)
    // This is useful to give prefix and suffix of the same file precisely the position necessary for FIM-like operation of a chat model

    let mut rebuilt_messages: Vec<ChatMessage> = post.messages.iter().take(user_msg_starts).map(|m| m.clone()).collect();
    for msg_idx in user_msg_starts..post.messages.len() {
        let mut user_posted = post.messages[msg_idx].content.clone();
        let user_posted_ntokens = count_tokens(&tokenizer.read().unwrap(), &user_posted);
        let mut context_limit = reserve_for_context / user_messages_with_at;
        if context_limit <= user_posted_ntokens {
            context_limit = 0;
        } else {
            context_limit -= user_posted_ntokens;
        }
        info!("msg {} user_posted {:?} that's {} tokens", msg_idx, user_posted, user_posted_ntokens);
        info!("that leaves {} tokens for context of this message", context_limit);

        let valid_commands = crate::at_commands::utils::find_valid_at_commands_in_query(&mut user_posted, &context).await;
        let mut messages_for_postprocessing = vec![];
        for cmd in valid_commands {
            match cmd.command.lock().await.execute(&user_posted, &cmd.args, top_n, &context).await {
                Ok(msg) => {
                    messages_for_postprocessing.push(msg);
                },
                Err(e) => {
                    tracing::warn!("can't execute command that indicated it can execute: {}", e);
                }
            }
        }
        let processed = postprocess_at_results(
            global_context.clone(),
            messages_for_postprocessing,
            tokenizer.clone(),
            context_limit
        ).await;
        let reloaded = reload_files(global_context.clone(), &processed, false).await;
        for msg in reloaded {
            rebuilt_messages.push(msg.clone());
            stream_back_to_user.push_in_json(json!(msg));
        }
        if user_posted.trim().len() > 0 {
            let msg = ChatMessage {
                role: "user".to_string(),
                content: user_posted,  // stream back to the user, without commands
            };
            rebuilt_messages.push(msg.clone());
            stream_back_to_user.push_in_json(json!(msg));
        }
    }
    post.messages = rebuilt_messages;
    user_msg_starts
}


pub struct HasVecdbResults {
    pub was_sent: bool,
    pub in_json: Vec<Value>,
}

impl HasVecdbResults {
    pub fn new() -> Self {
        HasVecdbResults {
            was_sent: false,
            in_json: vec![],
        }
    }
}

impl HasVecdbResults {
    pub fn push_in_json(&mut self, value: Value) {
        self.in_json.push(value);
    }

    pub fn response_streaming(&mut self) -> Result<Vec<Value>, String> {
        if self.was_sent == true || self.in_json.is_empty() {
            return Ok(vec![]);
        }
        self.was_sent = true;
        Ok(self.in_json.clone())
    }
}
