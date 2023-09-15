use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatPost;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock;

use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::info;

const DEBUG: bool = false;


#[derive(Debug)]
pub struct GenericChatScratchpad {
    pub t: HasTokenizerAndEot,
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
        GenericChatScratchpad { t: HasTokenizerAndEot::new(tokenizer), post, token_esc: "".to_string(), keyword_syst: "".to_string(), keyword_user: "".to_string(), keyword_asst: "".to_string(), default_system_message: "".to_string() }
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
        Ok(())
    }

    fn prompt(
        &self,
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let limit = context_size - self.post.parameters.max_new_tokens;
        let supports_stop = true; // TODO: take from model caps
        if supports_stop {
            let mut stop_list = vec![self.t.eot.clone()];
            if self.token_esc.len() > 0 {
                stop_list.push(self.token_esc.clone());
            }
            sampling_parameters_to_patch.stop = Some(stop_list);
        }
        let mut prompt = "".to_string();
        let mut message_token_count: Vec<usize> = vec![0; self.post.messages.len()];
        for (i, msg) in self.post.messages.iter().enumerate() {
            let cnt = 3 + self.t.count_tokens(msg.content.as_str())?;  // 3 for "\n\nASSISTANT:" kind of thing
        }
        // default_system_message
        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &self,
        choices: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        unimplemented!();
        // let tmp = choices.iter()
        //     .map(|x| {
        //         serde_json::json!({
        //             "code_completion": cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline).0.trim_end(),
        //         })
        //     }).collect::<Vec<_>>();
        // return Ok(serde_json::json!(tmp));
    }

    fn response_streaming(
        &self,
        delta: String,
    ) -> Result<(serde_json::Value, bool), String> {
        unimplemented!();
        // info!("delta: {}", delta);
        // // let mut finished = false;
        // let ans: serde_json::Value;
        // let (mut s, finished) = cut_result(&delta, self.t.eot.as_str(), self.post.inputs.multiline);
        // if finished {
        //     s = s.trim_end().to_string();
        // }
        // ans = serde_json::json!({
        //     "code_completion_delta": s
        // });
        // Ok((ans, finished))
    }
}


