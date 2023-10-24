use tracing::info;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;
use crate::call_validation::SamplingParameters;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history_in_bytes;
// use crate::vecdb_search::{VecdbSearch, embed_vecdb_results};
use crate::vecdb_search::VecdbSearch;


// const DEBUG: bool = true;


// #[derive(Debug)]
pub struct ChatPassthrough {
    pub post: ChatPost,
    pub default_system_message: String,
    pub limit_bytes: usize,
    pub vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
}

const DEFAULT_LIMIT_BYTES: usize = 4096*3;

impl ChatPassthrough {
    pub fn new(
        post: ChatPost,
        vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
    ) -> Self {
        ChatPassthrough {
            post,
            default_system_message: "".to_string(),
            limit_bytes: DEFAULT_LIMIT_BYTES,  // one token translates to 3 bytes (not unicode chars)
            vecdb_search,
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
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let limited_msgs: Vec<ChatMessage> = limit_messages_history_in_bytes(&self.post, self.limit_bytes, &self.default_system_message)?;
        info!("chat passthrough {} messages -> {} messages after applying limits and possibly adding the default system message", &limited_msgs.len(), &limited_msgs.len());
        let prompt = "MESSAGES ".to_string() + &serde_json::to_string(&limited_msgs).unwrap();
        Ok(prompt.to_string())
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        stopped: Vec<bool>,
    ) -> Result<serde_json::Value, String> {
        unimplemented!()
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        unimplemented!()
    }
}

