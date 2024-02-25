use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{error, info};
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history_in_bytes;
use crate::scratchpads::chat_utils_rag::{run_at_commands, HasVecdbResults};

const DEBUG: bool = true;


// #[derive(Debug)]
pub struct ChatPassthrough {
    pub post: ChatPost,
    pub default_system_message: String,
    pub limit_bytes: usize,
    pub has_vecdb_results: HasVecdbResults,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

const DEFAULT_LIMIT_BYTES: usize = 4096*6;

impl ChatPassthrough {
    pub fn new(
        post: ChatPost,
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Self {
        ChatPassthrough {
            post,
            default_system_message: "".to_string(),
            limit_bytes: DEFAULT_LIMIT_BYTES,  // one token translates to 3 bytes (not unicode chars)
            has_vecdb_results: HasVecdbResults::new(),
            global_context,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for ChatPassthrough {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String> {
        self.default_system_message = patch.get("default_system_message").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.limit_bytes = patch.get("limit_bytes").and_then(|x| x.as_u64()).unwrap_or(DEFAULT_LIMIT_BYTES as u64) as usize;
        Ok(())
    }

    async fn prompt(
        &mut self,
        _context_size: usize,
        _sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        info!("chat passthrough {} messages at start", &self.post.messages.len());
        let top_n = 6;
        run_at_commands(self.global_context.clone(), &mut self.post, top_n, &mut self.has_vecdb_results).await;
        let limited_msgs: Vec<ChatMessage> = limit_messages_history_in_bytes(&self.post.messages, self.limit_bytes, &self.default_system_message)?;
        info!("chat passthrough {} messages -> {} messages after applying at-commands and limits, possibly adding the default system message", &self.post.messages.len(), &limited_msgs.len());
        let mut filtered_msgs: Vec<ChatMessage> = Vec::<ChatMessage>::new();
        for msg in &limited_msgs {
            if msg.role == "assistant" || msg.role == "system" || msg.role == "user" {
                filtered_msgs.push(msg.clone());
            } else if msg.role == "context_file" {
                match serde_json::from_str(&msg.content) {
                    Ok(res) => {
                        let vector_of_context_files: Vec<ContextFile> = res;
                        for context_file in &vector_of_context_files {
                            filtered_msgs.push(ChatMessage {
                                role: "user".to_string(),
                                content: format!("{}:{}-{}\n```\n{}```",
                                    context_file.file_name,
                                    context_file.line1,
                                    context_file.line2,
                                    context_file.file_content),
                            });
                        }
                    },
                    Err(e) => { error!("error parsing context file: {}", e); }
                }
            }
        }
        let prompt = "PASSTHROUGH ".to_string() + &serde_json::to_string(&filtered_msgs).unwrap();
        if DEBUG {
            for msg in &filtered_msgs {
                info!("filtered message role={} {}", msg.role, crate::nicer_logs::first_n_chars(&msg.content, 40));
            }
        }
        Ok(prompt.to_string())
    }

    fn response_n_choices(
        &mut self,
        _choices: Vec<String>,
        _stopped: Vec<bool>,
    ) -> Result<serde_json::Value, String> {
        unimplemented!()
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        // info!("chat passthrough response_streaming delta={:?}, stop_toks={}, stop_length={}", delta, stop_toks, stop_length);
        let finished = stop_toks || stop_length;
        let json_choices;
        if finished {
            json_choices = serde_json::json!([{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": delta
                },
                "finish_reason": serde_json::Value::String(if stop_toks { "stop".to_string() } else { "length".to_string() }),
            }]);
        } else {
            json_choices = serde_json::json!([{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": delta
                },
                "finish_reason": serde_json::Value::Null
            }]);
        }
        let ans = serde_json::json!({
            "choices": json_choices,
        });
        Ok((ans, finished))
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        return self.has_vecdb_results.response_streaming();
    }
}
