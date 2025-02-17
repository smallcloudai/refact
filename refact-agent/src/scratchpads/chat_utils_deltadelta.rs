use serde_json::Value;
use crate::scratchpad_abstract::FinishReason;

#[derive(Debug)]
pub struct DeltaDeltaChatStreamer {
    // This class helps chat implementations to stop at two-token phrases (at most) when streaming,
    // by delaying output by 1 token.
    // (the problem is the naive approach would have already sent the first token to the user, instead of stopping)
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
        finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        assert!(!self.finished, "already finished");
        let mut json_choices = Vec::<Value>::new();
        for (i, x) in choices.iter().enumerate() {
            let s = cut_result(&x, &self.stop_list);
            json_choices.push(serde_json::json!({
                "index": i,
                "message": {
                    "role": self.role.clone(),
                    "content": s.clone()
                },
                "finish_reason": finish_reasons[i].to_string(),
            }));
        }
        Ok(serde_json::json!(
            {
                "choices": json_choices,
            }
        ))
    }

    pub fn response_streaming(&mut self, delta: String, finish_reason: FinishReason) -> Result<(Value, FinishReason), String> {
        // let prev_delta = self.delta2;
        assert!(!self.finished, "already finished");
        self.delta2 = self.delta1.clone();
        self.delta1 = delta.clone();
        let json_choices;
        if !delta.is_empty() {
            json_choices = serde_json::json!([{
                "index": 0,
                "delta": {
                    "role": self.role.clone(),
                    "content": self.delta2
                },
                "finish_reason": finish_reason.to_json_val()
            }]);
        } else {
            json_choices = serde_json::json!([{
                "index": 0,
                "delta": {
                    "role": self.role.clone(),
                    "content": self.delta2
                },
                "finish_reason": finish_reason.to_json_val()
            }]);
        }
        Ok((serde_json::json!({"choices": json_choices}), finish_reason))
    }

    pub fn streaming_finished(&mut self, finish_reason: FinishReason) -> Result<Value, String> {
        assert!(!self.finished, "already finished");
        self.finished = true;
        self.delta2 = self.delta1.clone();
        let leftovers = self.delta2.clone();
        Ok(serde_json::json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "role": self.role.clone(),
                    "content": cut_result(&leftovers, &self.stop_list),
                },
                "finish_reason": finish_reason.to_json_val()
            }],
        }))
    }
}

fn cut_result(text: &str, local_stop_list: &Vec<String>) -> String {
    let mut cut_at = vec![];
    for t in local_stop_list {
        if let Some(x) = text.find(t) {
            cut_at.push(x);
        }
    }
    if cut_at.is_empty() {
        return text.to_string().replace("\r", "");
    }
    let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    let ans = text.split_at(cut_at).0.to_string();
    ans.replace("\r", "")
}
