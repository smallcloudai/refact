use std::sync::Arc;
use std::cmp::Ordering;
use tracing::info;
use serde_json::{json, Value};
use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_commands::AtCommandsContext;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile};
use crate::global_context::GlobalContext;


const SMALL_GAP_LINES: i32 = 10;  // lines

pub fn postprocess_at_results(
    messages: Vec<ChatMessage>,
    max_bytes: usize,
) -> Vec<ChatMessage> {
    // 1. Decode all
    let mut cxfile_list: Vec<ContextFile> = vec![];
    for msg in messages {
        cxfile_list.extend(serde_json::from_str::<Vec<ContextFile>>(&msg.content).unwrap()); // TODO: this unwrap() is not good
    }
    // 2. Sort by usefullness
    cxfile_list.sort_by(|a, b| {
        b.usefullness.partial_cmp(&a.usefullness).unwrap_or(Ordering::Equal)
    });
    for cxfile in cxfile_list.iter() {
        info!("sorted file {}:{}-{} usefullness {}", crate::nicer_logs::last_n_chars(&cxfile.file_name, 40), cxfile.line1, cxfile.line2, cxfile.usefullness);
    }
    // 3. Truncate less useful to max_bytes
    let mut total_bytes: usize = cxfile_list.iter().map(|x| x.file_content.len()).sum();
    while total_bytes > max_bytes {
        let least_useful = cxfile_list.pop();
        match least_useful {
            Some(file) => {
                total_bytes -= file.file_content.len();
            },
            None => break,
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
                    info!("merging file {} range {}-{} with range {}-{}", x.file_name, x.line1, x.line2, y.line1, y.line2);
                    eaten[j] = true;
                    x.line1 = possible_merge_line1;
                    x.line2 = possible_merge_line2;
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
        info!("merged {}:{}-{}", cxfile_list[i].file_name, cxfile_list[i].line1, cxfile_list[i].line2);
    }
    // 5. Encode back into a single message
    let mut processed_messages: Vec<ChatMessage> = vec![];
    let message = ChatMessage {
        role: "context_file".to_string(),
        content: serde_json::to_string(&merged).unwrap(),
    };
    processed_messages.push(message);
    processed_messages
}

pub async fn run_at_commands(
    global_context: Arc<ARwLock<GlobalContext>>,
    post: &mut ChatPost,
    top_n: usize,
    has_vecdb: &mut HasVecdbResults,
) {
    // TODO: don't operate on `post`, return a copy of the messages
    let context = AtCommandsContext::new(global_context.clone()).await;
    // TODO: delete the last, not just take the last
    let mut query = post.messages.last().unwrap().content.clone(); // latest_msg_cont
    let valid_commands = crate::at_commands::utils::find_valid_at_commands_in_query(&mut query, &context).await;

    let mut messages_for_postprocessing = vec![];
    for cmd in valid_commands {
        match cmd.command.lock().await.execute(&query, &cmd.args, top_n, &context).await {
            Ok(msg) => {
                messages_for_postprocessing.push(msg);
            },
            Err(_) => {}
        }
    }
    let max_bytes = 5*1024;
    let processed = postprocess_at_results(
        messages_for_postprocessing,
        max_bytes
    );
    for msg in processed {
        post.messages.push(msg.clone());
        has_vecdb.push_in_json(json!(msg));
    }
    let msg = ChatMessage {
        role: "user".to_string(),
        content: query,  // stream back to the user, without commands
    };
    post.messages.push(msg.clone());
    has_vecdb.push_in_json(json!(msg));
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
