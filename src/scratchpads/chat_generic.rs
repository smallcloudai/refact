use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpads::chat_deltadelta::DeltaDeltaChatStreamer;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;
use crate::call_validation::SamplingParameters;
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
    pub role: String,
    pub reply_so_far: Vec<String>,
    pub finished_so_far: Vec<bool>,
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
            role: "".to_string(),
            reply_so_far: vec![],
            finished_so_far: vec![]
        }
    }
}

// PLAN 1 HOUR:
// + 1. History with tokens calc (function)
//   2. Streamer delta-delta class
//   3. Two classes with prompt() and glue
// => working llama2 chat AND refact chat
// 12:40


pub fn limit_messages_history(
    t: &HasTokenizerAndEot,
    post: &ChatPost,
    context_size: usize,
    default_system_mesage: String,
) -> Result<Vec<ChatMessage>, String>
{
    let tokens_limit: i32 = context_size as i32 - post.parameters.max_new_tokens as i32;
    let mut tokens_used: i32 = 0;
    let mut message_token_count: Vec<i32> = vec![0; post.messages.len()];
    let mut message_take: Vec<bool> = vec![false; post.messages.len()];
    let mut have_system = false;
    for (i, msg) in post.messages.iter().enumerate() {
        let tcnt = (3 + t.count_tokens(msg.content.as_str())?) as i32;  // 3 for role "\n\nASSISTANT:" kind of thing
        message_token_count[i] = tcnt;
        if i==0 && msg.role == "system" {
            message_take[i] = true;
            tokens_used += tcnt;
            have_system = true;
        }
    }
    if !have_system {
        let tcnt = default_system_mesage.len() as i32;
        tokens_used += tcnt;
    }
    for i in (0..post.messages.len()).rev() {
        let tcnt = message_token_count[i];
        if !message_take[i] {
            if tokens_used + tcnt < tokens_limit {
                message_take[i] = true;
                tokens_used += tcnt;
            }
        }
    }
    let mut messages_out: Vec<ChatMessage> = post.messages.iter().enumerate().filter(|(i, x)| message_take[*i]).map(|(_, x)| x.clone()).collect();
    if !have_system {
        messages_out.insert(0, ChatMessage {
            role: "system".to_string(),
            content: default_system_mesage.clone(),
        });
    }
    Ok(messages_out)
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
        let limit = context_size - self.post.parameters.max_new_tokens;
        let mut stop_list = vec![self.t.eot.clone()];
        if self.token_esc.len() > 0 {
            stop_list.push(self.token_esc.clone());
        }
        sampling_parameters_to_patch.stop = Some(stop_list);
        let mut prompt = "".to_string();
        prompt = "<empty_output>USER pygame example\n\n<empty_output>ASSISTANT".to_string();
        self.role = "assistant".to_string();
        // default_system_message
        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
        }
        self.dd.role = self.role.clone();
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        self.dd.response_n_choices(choices)
    }

    fn response_streaming(
        &mut self,
        delta: String,
    ) -> Result<(serde_json::Value, bool), String> {
        self.dd.response_streaming(delta)
    }
}

