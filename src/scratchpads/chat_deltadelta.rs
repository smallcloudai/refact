use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock;

use tokenizers::Tokenizer;
use tracing::info;


#[derive(Debug)]
pub struct DeltaDeltaChatStreamer {
    // This class helps chat implementations to stop at two-token phrases (at most) when streaming,
    // by delaying output by 1 token.
    pub delta1: String,
    pub delta2: String,
    pub finished: bool,
    pub stop_list: Vec<String>,
    pub role: String,
}

impl DeltaDeltaChatStreamer {
    pub fn new() -> Self {
        Self {
            delta1: String::new(),
            delta2: String::new(),
            finished: false,
            stop_list: Vec::new(),
            role: String::new(),
        }
    }

    pub fn response_n_choices(
        &mut self,
        choices: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        assert!(!self.finished, "already finished");
        info!("response_n_choices: {:?}", choices);
        let mut json_choices = Vec::<serde_json::Value>::new();
        for (i, x) in choices.iter().enumerate() {
            let (s, finished) = cut_result(&x, &self.stop_list);
            json_choices.push(serde_json::json!({
                "index": i,
                "message": {
                    "role": self.role.clone(),
                    "content": s.clone()
                },
                "finish_reason": (if finished { "stop" } else { "length" }).to_string(),
            }));
        }
        Ok(serde_json::json!(
            {
                "choices": json_choices,
            }
        ))
    }

    pub fn response_streaming(
        &mut self,
        delta: String,
    ) -> Result<(serde_json::Value, bool), String> {
        assert!(!self.finished, "already finished");
        // let prev_delta = self.delta2;
        self.delta2 = self.delta1.clone();
        self.delta1 = delta.clone();
        info!("delta2 {:?} delta1 {:?}", self.delta2, self.delta1);
        let finished;
        let json_choices;
        if !delta.is_empty() {
            let big_delta = self.delta1.clone() + self.delta2.as_str();
            let s: String;
            (s, finished) = cut_result(&big_delta, &self.stop_list);
            if finished {
                json_choices = serde_json::json!([{
                    "index": 0,
                    "delta": {
                        "role": self.role.clone(),
                        "content": s.clone(),
                    },
                    "finish_reason": serde_json::Value::String("stop".to_string()),
                }]);
            } else {
                json_choices = serde_json::json!([{
                    "index": 0,
                    "delta": {
                        "role": self.role.clone(),
                        "content": self.delta2
                    },
                    "finish_reason": serde_json::Value::Null
                }]);
            }
        } else {
            let leftovers = self.delta2.clone();
            let s: String;
            (s, finished) = cut_result(&leftovers, &self.stop_list);
            if finished {
                json_choices = serde_json::json!([{
                    "index": 0,
                    "delta": {
                        "role": self.role.clone(),
                        "content": s.clone(),
                    },
                    "finish_reason": serde_json::Value::String("stop".to_string()),
                }]);
            } else {
                json_choices = serde_json::json!([{
                    "index": 0,
                    "delta": {
                        "role": self.role.clone(),
                        "content": self.delta2
                    },
                    "finish_reason": "length"
                }]);
            }
        }
        self.finished = finished;
        let ans = serde_json::json!({
            "choices": json_choices,
        });
        Ok((ans, finished))
    }
}

fn cut_result(
    text: &str,
    local_stop_list: &Vec<String>,
) -> (String, bool) {
    let mut cut_at = vec![];
    for t in local_stop_list {
        if let Some(x) = text.find(t) {
            cut_at.push(x);
        }
    }
    if cut_at.is_empty() {
        return (text.to_string().replace("\r", ""), false);
    }
    let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    let ans = text.split_at(cut_at).0.to_string();
    return (ans.replace("\r", ""), true);
}

