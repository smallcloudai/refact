use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile, SamplingParameters};
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history_in_bytes;
use crate::scratchpads::chat_utils_rag::{chat_functions_middleware, HasVecdb, HasVecdbResults};
use crate::vecdb::structs::VecdbSearch;

const DEBUG: bool = true;


// #[derive(Debug)]
pub struct ChatPassthrough<T> {
    pub post: ChatPost,
    pub default_system_message: String,
    pub limit_bytes: usize,
    pub vecdb_search: Arc<AMutex<Option<T>>>,
    pub has_vecdb_results: HasVecdbResults,
}

const DEFAULT_LIMIT_BYTES: usize = 4096*6;

impl<T: Send + Sync + VecdbSearch> ChatPassthrough<T> {
    pub fn new(
        post: ChatPost,
        vecdb_search: Arc<AMutex<Option<T>>>,
    ) -> Self where T: VecdbSearch + 'static + Sync {
        ChatPassthrough {
            post,
            default_system_message: "".to_string(),
            limit_bytes: DEFAULT_LIMIT_BYTES,  // one token translates to 3 bytes (not unicode chars)
            vecdb_search,
            has_vecdb_results: HasVecdbResults::new(),
        }
    }
}

#[async_trait]
impl<T: Send + Sync + VecdbSearch> ScratchpadAbstract for ChatPassthrough<T> {
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
        match *self.vecdb_search.lock().await {
            Some(ref db) => chat_functions_middleware(db, &mut self.post, 6, &mut self.has_vecdb_results).await,
            None => {}
        }
        let limited_msgs: Vec<ChatMessage> = limit_messages_history_in_bytes(&self.post.messages, self.limit_bytes, &self.default_system_message)?;
        info!("chat passthrough {} messages -> {} messages after applying limits and possibly adding the default system message", &limited_msgs.len(), &limited_msgs.len());
        let mut filtered_msgs: Vec<ChatMessage> = Vec::<ChatMessage>::new();
        for msg in &limited_msgs {
            if msg.role == "assistant" || msg.role == "system" || msg.role == "user" {
                filtered_msgs.push(msg.clone());
            } else if msg.role == "context_file" {
                let vector_of_context_files: Vec<ContextFile> = serde_json::from_str(&msg.content).unwrap(); // FIXME unwrap
                for context_file in &vector_of_context_files {
                    filtered_msgs.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("{}\n```\n{}```", context_file.file_name, context_file.file_content),
                    });
                }
            }
        }
        let prompt = "PASSTHROUGH ".to_string() + &serde_json::to_string(&filtered_msgs).unwrap();
        if DEBUG {
            for msg in &filtered_msgs {
                info!("filtered message: {:?}", msg);
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
    fn response_spontaneous(&mut self) -> Result<serde_json::Value, String> {
        return self.has_vecdb_results.response_streaming();
    }
}
