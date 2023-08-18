use tracing::info;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::Mutex as AMutex;
use tokenizers::Tokenizer;
use async_trait::async_trait;

use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::chat_utils_deltadelta::DeltaDeltaChatStreamer;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;
use crate::call_validation::SamplingParameters;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use crate::vecdb_search::{VecdbSearch, embed_vecdb_results};


const DEBUG: bool = true;


// #[derive(Debug)]
pub struct ChatLlama2 {
    pub t: HasTokenizerAndEot,
    pub dd: DeltaDeltaChatStreamer,
    pub post: ChatPost,
    pub keyword_s: String, // "SYSTEM:" keyword means it's not one token
    pub keyword_slash_s: String,
    pub default_system_message: String,
    pub vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
}


impl ChatLlama2 {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: ChatPost,
        vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
    ) -> Self {
        ChatLlama2 {
            t: HasTokenizerAndEot::new(tokenizer),
            dd: DeltaDeltaChatStreamer::new(),
            post,
            keyword_s: "<s>".to_string(),
            keyword_slash_s: "</s>".to_string(),
            default_system_message: "".to_string(),
            vecdb_search
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for ChatLlama2 {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String> {
        self.keyword_s = patch.get("s").and_then(|x| x.as_str()).unwrap_or("<s>").to_string();
        self.keyword_slash_s = patch.get("slash_s").and_then(|x| x.as_str()).unwrap_or("</s>").to_string();
        self.default_system_message = patch.get("default_system_message").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.t.eot = self.keyword_s.clone();
        info!("llama2 chat model adaptation patch applied {:?}", self.keyword_s);
        self.t.assert_one_token(&self.t.eot.as_str())?;
        self.dd.stop_list.clear();
        self.dd.stop_list.push(self.t.eot.clone());
        self.dd.stop_list.push(self.keyword_slash_s.clone());
        Ok(())
    }

    async fn prompt(
        &mut self,
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        embed_vecdb_results(self.vecdb_search.clone(), &mut self.post, 3).await;
        let limited_msgs: Vec<ChatMessage> = limit_messages_history(&self.t, &self.post, context_size, &self.default_system_message)?;
        sampling_parameters_to_patch.stop = Some(self.dd.stop_list.clone());
        // loosely adapted from https://huggingface.co/spaces/huggingface-projects/llama-2-13b-chat/blob/main/model.py#L24
        let mut prompt = "".to_string();
        prompt.push_str(self.keyword_s.as_str());
        prompt.push_str("[INST] ");
        let mut do_strip = false;
        for msg in limited_msgs {
            if msg.role == "system" {
                if !do_strip {
                    prompt.push_str("<<SYS>>\n");
                    prompt.push_str(self.default_system_message.as_str());
                    prompt.push_str("\n<</SYS>>\n");
                }
            } else {
                // prompt.push_str("\n\n");
            }
            if msg.role == "user" {
                let user_input = if do_strip { msg.content.trim().to_string() } else { msg.content.clone() };
                prompt.push_str(user_input.as_str());
                prompt.push_str(" [/INST]");
                do_strip = true;
            }
            if msg.role == "assistant" {
                prompt.push_str(msg.content.trim());
                prompt.push_str(" ");
                prompt.push_str(&self.keyword_slash_s.as_str());
                prompt.push_str(&self.keyword_s.as_str());
                prompt.push_str("[INST]");
            }
        }
        // This only supports assistant, not suggestions for user
        self.dd.role = "assistant".to_string();
        if DEBUG {
            // info!("llama2 chat vdb_suggestion {:?}", vdb_suggestion);
            info!("llama2 chat prompt\n{}", prompt);
            info!("llama2 chat re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        stopped: Vec<bool>,
    ) -> Result<serde_json::Value, String> {
        self.dd.response_n_choices(choices, stopped)
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        self.dd.response_streaming(delta, stop_toks)
    }
}

