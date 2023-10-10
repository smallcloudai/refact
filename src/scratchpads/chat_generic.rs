use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::chat_utils_deltadelta::DeltaDeltaChatStreamer;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;
use crate::call_validation::SamplingParameters;
use crate::scratchpads::chat_utils_limit_history::limit_messages_history;
use std::sync::Arc;
use std::sync::RwLock;

use tokenizers::Tokenizer;
use tracing::info;

const DEBUG: bool = true;


#[derive(Debug)]
pub struct GenericChatScratchpad {
    pub t: HasTokenizerAndEot,
    pub dd: DeltaDeltaChatStreamer,
    pub post: ChatPost,
    pub token_esc: String,    // for models that switch between sections using <esc>SECTION
    pub keyword_syst: String, // "SYSTEM:" keyword means it's not one token
    pub keyword_user: String,
    pub keyword_asst: String,
    pub default_system_message: String,
}

impl GenericChatScratchpad {
    pub fn new(
        tokenizer: Arc<RwLock<Tokenizer>>,
        post: ChatPost,
    ) -> Self {
        GenericChatScratchpad {
            t: HasTokenizerAndEot::new(tokenizer),
            dd: DeltaDeltaChatStreamer::new(),
            post,
            token_esc: "".to_string(),
            keyword_syst: "".to_string(),
            keyword_user: "".to_string(),
            keyword_asst: "".to_string(),
            default_system_message: "".to_string(),
        }
    }
}

impl ScratchpadAbstract for GenericChatScratchpad {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String> {
        self.token_esc = patch.get("token_esc").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.keyword_syst = patch.get("keyword_system").and_then(|x| x.as_str()).unwrap_or("SYSTEM:").to_string();
        self.keyword_user = patch.get("keyword_user").and_then(|x| x.as_str()).unwrap_or("USER:").to_string();
        self.keyword_asst = patch.get("keyword_assistant").and_then(|x| x.as_str()).unwrap_or("ASSISTANT:").to_string();
        self.default_system_message = patch.get("default_system_message").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.assert_one_token(&self.t.eot.as_str())?;
        self.dd.stop_list.clear();
        self.dd.stop_list.push(self.t.eot.clone());
        if self.token_esc.len() > 0 {
            self.dd.stop_list.push(self.token_esc.clone());
        } else {
            self.dd.stop_list.push(self.keyword_syst.clone());
            self.dd.stop_list.push(self.keyword_user.clone());
            self.dd.stop_list.push(self.keyword_asst.clone());
        }
        Ok(())
    }

    fn prompt(
        &mut self,
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let limited_msgs: Vec<ChatMessage> = limit_messages_history(&self.t, &self.post, context_size, &self.default_system_message)?;
        sampling_parameters_to_patch.stop = Some(self.dd.stop_list.clone());
        // adapted from https://huggingface.co/spaces/huggingface-projects/llama-2-13b-chat/blob/main/model.py#L24
        let mut prompt = "".to_string();
        let mut last_role = "assistant".to_string();
        for msg in limited_msgs {
            if self.token_esc.len() > 0 {
                prompt.push_str(self.token_esc.as_str());
                if msg.role == "system" {
                    prompt.push_str(self.keyword_syst.as_str());
                } else if msg.role == "user" {
                    prompt.push_str(self.keyword_user.as_str());
                } else if msg.role == "assistant" {
                    prompt.push_str(self.keyword_asst.as_str());
                } else {
                    return Err(format!("role \"{}\"not recognized", msg.role));
                }
                last_role = msg.role.clone();
            }
            prompt.push_str(" ");
            prompt.push_str(msg.content.as_str());
            prompt.push_str("\n\n");
        }
        if last_role == "assistant" || last_role == "system" {
            self.dd.role = "user".to_string();
            prompt.push_str(self.keyword_user.as_str());
        } else if last_role == "user" {
            self.dd.role = "assistant".to_string();
            prompt.push_str(self.keyword_asst.as_str());
        }
        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
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

