use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::CodeCompletionPost;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock;

// use ropey::RopeSlice;
use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::info;

const DEBUG: bool = false;


#[derive(Debug)]
pub struct SingleFileFIM {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub order: String,
    pub fim_prefix: String,
    pub fim_suffix: String,
    pub fim_middle: String,
}

impl SingleFileFIM {
    pub fn new(
        tokenizer: Arc<RwLock<Tokenizer>>,
        post: CodeCompletionPost,
        order: String,
    ) -> Self {
        SingleFileFIM { t: HasTokenizerAndEot::new(tokenizer), post, order, fim_prefix: String::new(), fim_suffix: String::new(), fim_middle: String::new() }
    }
}


impl ScratchpadAbstract for SingleFileFIM {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String> {
        // That will work for some models (starcoder) without patching
        self.fim_prefix = patch.get("fim_prefix").and_then(|x| x.as_str()).unwrap_or("<fim_prefix>").to_string();
        self.fim_suffix = patch.get("fim_suffix").and_then(|x| x.as_str()).unwrap_or("<fim_suffix>").to_string();
        self.fim_middle = patch.get("fim_middle").and_then(|x| x.as_str()).unwrap_or("<fim_middle>").to_string();
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.assert_one_token(&self.fim_prefix.as_str())?;
        self.t.assert_one_token(&self.fim_suffix.as_str())?;
        self.t.assert_one_token(&self.fim_middle.as_str())?;
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
            let mut stop_list = vec![self.t.eot.clone(), "\n\n".to_string()];
            if !self.post.inputs.multiline {
                stop_list.push("\n".to_string());  // This doesn't stop hf inference, only whole tokens do
            }
            sampling_parameters_to_patch.stop = Some(stop_list);
        }
        let text = Rope::from_str(
            self.post.inputs.sources.get(&self.post.inputs.cursor.file)
            .ok_or("Cursor is in file not found in sources".to_string())?
        );
        let pos = &self.post.inputs.cursor;
        let mut before_iter = text.lines_at(pos.line as usize).reversed();
        let mut after_iter = text.lines_at(pos.line as usize + 1);

        let mut before_line = before_iter.next();

        let cursor_line1: String;
        let col = pos.character as usize;
        cursor_line1 = text.line(pos.line as usize).slice(0..col).to_string();
        // UNFINISHED LI|

        let mut after_line = after_iter.next();

        let cursor_line2: String;
        if self.post.inputs.multiline {
            cursor_line2 = text.line(pos.line as usize).slice(col..).to_string();
        } else {
            cursor_line2 = "".to_string();
        }

        let mut before = vec![];
        let mut after = String::new();
        let mut tokens_used = self.t.count_tokens(
            (cursor_line1.clone() + &cursor_line2).as_str()
        )?;
        while before_line.is_some() || after_line.is_some() {
            if let Some(before_line) = before_line {
                let before_line = before_line.to_string();
                let tokens = self.t.count_tokens(before_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                before.push(before_line);
            }
            if let Some(after_line) = after_line {
                let after_line = after_line.to_string();
                let tokens = self.t.count_tokens(after_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                after.push_str(&after_line);
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }
        info!("single file FIM prompt {} tokens used < limit {}", tokens_used, limit);
        let prompt: String;
        if self.order == "PSM" {
            prompt = format!(
                "{}{}{}{}{}{}{}",
                self.fim_prefix,
                before.into_iter().rev().collect::<Vec<_>>().join(""),
                cursor_line1,
                self.fim_suffix,
                cursor_line2,
                after,
                self.fim_middle
            );
        } else if self.order == "SPM" {
            prompt = format!(
                "{}{}{}{}{}{}{}",
                self.fim_suffix,
                cursor_line2,
                after,
                self.fim_prefix,
                before.into_iter().rev().collect::<Vec<_>>().join(""),
                cursor_line1,
                self.fim_middle,
            );
        } else {
            return Err(format!("order \"{}\" not recognized", self.order));
        }
        if DEBUG {
            info!("prompt\n{}", prompt);
            info!("re-encode whole prompt again gives {} tokes", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &self,
        choices: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        let tmp = choices.iter()
            .map(|x| {
                serde_json::json!({
                    "code_completion": cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline).0.trim_end(),
                })
            }).collect::<Vec<_>>();
        return Ok(serde_json::json!(tmp));
    }

    fn response_streaming(
        &self,
        delta: String,
    ) -> Result<(serde_json::Value, bool), String> {
        info!("delta: {}", delta);
        // let mut finished = false;
        let ans: serde_json::Value;
        let (mut s, finished) = cut_result(&delta, self.t.eot.as_str(), self.post.inputs.multiline);
        if finished {
            s = s.trim_end().to_string();
        }
        ans = serde_json::json!({
            "code_completion_delta": s
        });
        Ok((ans, finished))
    }
}


fn cut_result(text: &str, eot_token: &str, multiline: bool) -> (String, bool) {
    let mut cut_at = vec![];
    if let Some(x) = text.find(eot_token) {
        cut_at.push(x);
    }
    if let Some(x) = text.find("\n\n") {
        cut_at.push(x);
    }
    if !multiline {
        if let Some(x) = text.find("\n") {
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

