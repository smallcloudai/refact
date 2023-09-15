use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatPost;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock;

use tokenizers::Tokenizer;
use tracing::info;

const DEBUG: bool = true;


#[derive(Debug)]
pub struct GenericChatScratchpad {
    pub t: HasTokenizerAndEot,
    pub post: ChatPost,
    pub token_esc: String,    // for models that switch between sections using <esc>SECTION
    pub keyword_syst: String, // "SYSTEM:" keyword means it's not one token
    pub keyword_user: String,
    pub keyword_asst: String,
    pub default_system_message: String,
    pub local_stop_list: Vec<String>,
    pub role: String,
    pub reply_so_far: Vec<String>,
}

impl GenericChatScratchpad {
    pub fn new(
        tokenizer: Arc<RwLock<Tokenizer>>,
        post: ChatPost,
    ) -> Self {
        GenericChatScratchpad { t: HasTokenizerAndEot::new(tokenizer), post, token_esc: "".to_string(), keyword_syst: "".to_string(), keyword_user: "".to_string(), keyword_asst: "".to_string(), default_system_message: "".to_string(), local_stop_list: vec![], role: "".to_string(), reply_so_far: vec![] }
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
        &mut self,
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
        self.local_stop_list.clear();
        self.local_stop_list.push(self.t.eot.clone());
        if self.token_esc.len() > 0 {
            self.local_stop_list.push(self.token_esc.clone());
        } else {
            self.local_stop_list.push(self.keyword_syst.clone());
            self.local_stop_list.push(self.keyword_user.clone());
            self.local_stop_list.push(self.keyword_asst.clone());
        }

        let mut prompt = "".to_string();
        let mut message_token_count: Vec<usize> = vec![0; self.post.messages.len()];
        for (i, msg) in self.post.messages.iter().enumerate() {
            let cnt = 3 + self.t.count_tokens(msg.content.as_str())?;  // 3 for "\n\nASSISTANT:" kind of thing
        }
        prompt = "USER: pygame example\n\nASSISTANT:".to_string();
        self.role = "assistant".to_string();
        // default_system_message
        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        info!("choices: {:?}", choices);
        // Should return:
        //   {
        //     "id": "chatcmpl-7yxWTrDK6x82DoFS7eVWeWmq8aXUp",
        //     "object": "chat.completion",
        //     "created": 1694762841,
        //     "model": "gpt-3.5-turbo-0613",
        //     "choices": [
        //       {
        //         "index": 0,
        //         "message": {
        //           "role": "assistant",
        //           "content": "Hello! Sure, here's a simple test program"
        //         },
        //         "finish_reason": "length"
        //       }
        //     ],
        //     "usage": {
        //       "prompt_tokens": 43,
        //       "completion_tokens": 10,
        //       "total_tokens": 53
        //     }
        //   }
        // data: {"object": "text_completion", "choices": [{"index": 0, "finish_reason": "", "role": "assistant", "delta": " be located in the same directory as the Dockerfile"}]}

        // self.reply_so_far.resize(choices.len(), "".to_string());
        // for (i, x) in choices.iter().enumerate() {
        //     let (s, finished) = cut_result(&x, &self.local_stop_list);
        //     self.reply_so_far[i] = x.clone();
        // }

        let tmp = self.reply_so_far.iter().enumerate()
            .map(|(i, x)| {
                serde_json::json!({
                    "index": i,
                    "message": {
                        "role": self.role.clone(),
                        "content": x.clone()
                    }
                })
            }).collect::<Vec<_>>();
        return Ok(serde_json::json!(tmp));
    }

    fn response_streaming(
        &mut self,
        delta: String,
    ) -> Result<(serde_json::Value, bool), String> {
        unimplemented!();
        // self.reply_so_far.resize(choices.len(), "".to_string())
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


fn cut_result(
    text: &str,
    local_stop_list: &Vec<String>,
) -> (String, bool) {
    // let mut cut_at = vec![];
    // if let Some(x) = text.find(eot_token) {
    //     cut_at.push(x);
    // }
    // if let Some(x) = text.find("\n\n") {
    //     cut_at.push(x);
    // }
    // if !multiline {
    //     if let Some(x) = text.find("\n") {
    //         cut_at.push(x);
    //     }
    // }
    // if cut_at.is_empty() {
    //     return (text.to_string().replace("\r", ""), false);
    // }
    // let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    // let ans = text.split_at(cut_at).0.to_string();
    // return (ans.replace("\r", ""), true);
    return (text.to_string(), false);
}

