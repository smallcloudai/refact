use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use async_trait::async_trait;
use serde_json::Value;
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tracing::{info, error};
use crate::at_commands::execute_at::run_at_commands;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpads::chat_generic::default_system_message_from_patch;
use crate::scratchpads::chat_utils_deltadelta::DeltaDeltaChatStreamer;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use crate::scratchpads::chat_utils_rag::HasRagResults;

const DEBUG: bool = true;


// #[derive(Debug)]
pub struct ChatLlama2 {
    pub t: HasTokenizerAndEot,
    pub dd: DeltaDeltaChatStreamer,
    pub post: ChatPost,
    pub keyword_s: String, // "SYSTEM:" keyword means it's not one token
    pub keyword_slash_s: String,
    pub default_system_message: String,
    pub has_rag_results: HasRagResults,
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub allow_at: bool,
}


impl ChatLlama2 {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: &ChatPost,
        global_context: Arc<ARwLock<GlobalContext>>,
        allow_at: bool,
    ) -> Self {
        ChatLlama2 {
            t: HasTokenizerAndEot::new(tokenizer),
            dd: DeltaDeltaChatStreamer::new(),
            post: post.clone(),
            keyword_s: "<s>".to_string(),
            keyword_slash_s: "</s>".to_string(),
            default_system_message: "".to_string(),
            has_rag_results: HasRagResults::new(),
            global_context,
            allow_at,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for ChatLlama2 {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
        exploration_tools: bool,
    ) -> Result<(), String> {
        self.keyword_s = patch.get("s").and_then(|x| x.as_str()).unwrap_or("<s>").to_string();
        self.keyword_slash_s = patch.get("slash_s").and_then(|x| x.as_str()).unwrap_or("</s>").to_string();
        self.default_system_message = default_system_message_from_patch(&patch, self.global_context.clone(), exploration_tools).await;
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
        let top_n: usize = 7;
        let (messages, undroppable_msg_n, _any_context_produced) = if self.allow_at {
            run_at_commands(self.global_context.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, context_size, &self.post.messages, top_n, &mut self.has_rag_results).await
        } else {
            (self.post.messages.clone(), self.post.messages.len(), false)
        };
        let limited_msgs: Vec<ChatMessage> = limit_messages_history(&self.t, &messages, undroppable_msg_n, sampling_parameters_to_patch.max_new_tokens, context_size, &self.default_system_message)?;
        sampling_parameters_to_patch.stop = self.dd.stop_list.clone();
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
            if msg.role == "context_file" {
                let vector_of_context_files: Vec<ContextFile> = serde_json::from_str(&msg.content).map_err(|e|error!("parsing context_files has failed: {}; content: {}", e, &msg.content)).unwrap_or_default();
                for context_file in vector_of_context_files {
                    prompt.push_str(format!("{}\n```\n{}```\n\n", context_file.file_name, context_file.file_content).as_str());
                }
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
            info!("llama2 chat re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
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
        _stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        self.dd.response_streaming(delta, stop_toks)
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        return self.has_rag_results.response_streaming();
    }
}

