use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tracing::{info, error};

use crate::at_commands::execute_at::run_at_commands_locally;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatPost, ContextFile, SamplingParameters};
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot, ScratchpadAbstract};
use crate::scratchpads::chat_utils_deltadelta::DeltaDeltaChatStreamer;
use crate::scratchpads::chat_utils_limit_history::fix_and_limit_messages_history;
use crate::scratchpads::chat_utils_prompts::prepend_the_right_system_prompt_and_maybe_more_initial_messages;
use crate::scratchpads::scratchpad_utils::HasRagResults;


const DEBUG: bool = true;


pub struct GenericChatScratchpad {
    pub t: HasTokenizerAndEot,
    pub dd: DeltaDeltaChatStreamer,
    #[allow(dead_code)]
    pub post: ChatPost,
    pub messages: Vec<ChatMessage>,
    pub token_bos: String,
    pub token_esc: String,
    // for models that switch between sections using <esc>SECTION
    pub keyword_syst: String,
    // "SYSTEM:" keyword means it's not one token
    pub keyword_user: String,
    pub keyword_asst: String,
    pub prepend_system_prompt: bool,
    pub has_rag_results: HasRagResults,
    pub allow_at: bool,
}

impl GenericChatScratchpad {
    pub fn new(
        tokenizer: Option<Arc<Tokenizer>>,
        post: &ChatPost,
        messages: &Vec<ChatMessage>,
        prepend_system_prompt: bool,
        allow_at: bool,
    ) -> Self {
        GenericChatScratchpad {
            t: HasTokenizerAndEot::new(tokenizer),
            dd: DeltaDeltaChatStreamer::new(),
            post: post.clone(),
            messages: messages.clone(),
            token_bos: "".to_string(),
            token_esc: "".to_string(),
            keyword_syst: "".to_string(),
            keyword_user: "".to_string(),
            keyword_asst: "".to_string(),
            prepend_system_prompt,
            has_rag_results: HasRagResults::new(),
            allow_at,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for GenericChatScratchpad {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String> {
        self.token_bos = patch.get("token_bos").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.token_esc = patch.get("token_esc").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.keyword_syst = patch.get("keyword_system").and_then(|x| x.as_str()).unwrap_or("SYSTEM:").to_string();
        self.keyword_user = patch.get("keyword_user").and_then(|x| x.as_str()).unwrap_or("USER:").to_string();
        self.keyword_asst = patch.get("keyword_assistant").and_then(|x| x.as_str()).unwrap_or("ASSISTANT:").to_string();

        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();

        self.dd.stop_list.clear();
        if !self.t.eot.is_empty() {
            self.t.assert_one_token(&self.t.eot.as_str())?;
            self.dd.stop_list.push(self.t.eot.clone());
        }
        if self.token_esc.len() > 0 {
            self.dd.stop_list.push(self.token_esc.clone());
        } else {
            self.dd.stop_list.push(self.keyword_syst.clone());
            self.dd.stop_list.push(self.keyword_user.clone());
            self.dd.stop_list.push(self.keyword_asst.clone());
        }
        self.dd.stop_list.retain(|x| !x.is_empty());

        Ok(())
    }

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let (gcx, n_ctx, should_execute_remotely) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.global_context.clone(), ccx_locked.n_ctx, ccx_locked.should_execute_remotely)
        };

        let messages = if self.prepend_system_prompt && self.allow_at {
            prepend_the_right_system_prompt_and_maybe_more_initial_messages(gcx.clone(), self.messages.clone(), &self.post.meta, &mut self.has_rag_results).await
        } else {
            self.messages.clone()
        };
        let (messages, _any_context_produced) = if self.allow_at && !should_execute_remotely {
            run_at_commands_locally(ccx.clone(), self.t.tokenizer.clone(), sampling_parameters_to_patch.max_new_tokens, &messages, &mut self.has_rag_results).await
        } else {
            (self.messages.clone(), false)
        };
        let (limited_msgs, _compression_strength) = fix_and_limit_messages_history(&self.t, &messages, sampling_parameters_to_patch, n_ctx, None, self.post.model.as_str())?;
        // if self.supports_tools {
        // };
        sampling_parameters_to_patch.stop = self.dd.stop_list.clone();
        // adapted from https://huggingface.co/spaces/huggingface-projects/llama-2-13b-chat/blob/main/model.py#L24
        let mut prompt = self.token_bos.to_string();
        let mut last_role = "assistant".to_string();
        for msg in limited_msgs {
            let content_text_only = msg.content.content_text_only();
            prompt.push_str(self.token_esc.as_str());
            if msg.role == "system" {
                prompt.push_str(self.keyword_syst.as_str());
                prompt.push_str(content_text_only.as_str());
                prompt.push_str("\n");
            } else if msg.role == "user" {
                prompt.push_str(self.keyword_user.as_str());
                prompt.push_str(content_text_only.as_str());
                prompt.push_str("\n");
            } else if msg.role == "cd_instruction" {
                prompt.push_str(self.keyword_user.as_str());
                prompt.push_str(content_text_only.as_str());
                prompt.push_str("\n");
            } else if msg.role == "assistant" {
                prompt.push_str(self.keyword_asst.as_str());
                prompt.push_str(content_text_only.as_str());
                prompt.push_str("\n");
            } else if msg.role == "context_file" {
                let vector_of_context_files: Vec<ContextFile> = serde_json::from_str(&content_text_only).map_err(|e|error!("parsing context_files has failed: {}; content: {}", e, &msg.content.content_text_only())).unwrap_or(vec![]);
                for context_file in vector_of_context_files {
                    prompt.push_str(format!("{}\n```\n{}```\n\n", context_file.file_name, context_file.file_content).as_str());
                }
            } else {
                return Err(format!("role \"{}\"not recognized", msg.role));
            }
            last_role = msg.role.clone();
            prompt.push_str(self.token_esc.as_str());
        }
        prompt.push_str(self.token_esc.as_str());
        if last_role == "assistant" || last_role == "system" {
            self.dd.role = "user".to_string();
            prompt.push_str(self.keyword_user.as_str());
        } else if last_role == "user" {
            self.dd.role = "assistant".to_string();
            prompt.push_str(self.keyword_asst.as_str());
        }
        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        self.dd.response_n_choices(choices, finish_reasons)
    }

    fn response_streaming(
        &mut self,
        delta: String,
        finish_reason: FinishReason
    ) -> Result<(Value, FinishReason), String> {
        self.dd.response_streaming(delta, finish_reason)
    }

    fn response_message_streaming(
        &mut self,
        _delta: &Value,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String> {
        self.has_rag_results.response_streaming()
    }

    fn streaming_finished(&mut self, finish_reason: FinishReason) -> Result<Value, String> {
        self.dd.streaming_finished(finish_reason)
    }
}
