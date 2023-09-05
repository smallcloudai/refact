use crate::scratchpads::scratchpad_abstract::CodeCompletionScratchpad;
use crate::scratchpads::call_validation::CodeCompletionPost;
use std::sync::Arc;
use std::sync::RwLock;

// use ropey::RopeSlice;
use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::{info, error};


const DEBUG: bool = true;


#[derive(Debug)]
pub struct SingleFileFIM {
    pub tokenizer: Arc<RwLock<Tokenizer>>,
    pub post: CodeCompletionPost,
}

impl SingleFileFIM {
    pub fn new(
        tokenizer: Arc<RwLock<Tokenizer>>,
        post: CodeCompletionPost,
    ) -> Self {
        SingleFileFIM { tokenizer, post }
    }
}

impl CodeCompletionScratchpad for SingleFileFIM {
    fn prompt(
        &self,
        context_size: usize,
    ) -> Result<String, String> {
        // TODO: assert one token
        let fim_prefix = "<fim_prefix>";
        let fim_suffix = "<fim_suffix>";
        let fim_middle = "<fim_middle>";
        let text_cursor_file_maybe = self.post.inputs.sources.get(&self.post.inputs.cursor.file);
        let text = match text_cursor_file_maybe {
            Some(x) => Rope::from_str(x),
            None => {
                return Err("Cursor is in file not found in sources".to_string());
            }
        };
        let mut token_count = context_size;
        let pos = &self.post.inputs.cursor;
        let mut before_iter = text.lines_at(pos.line as usize).reversed();
        let mut after_iter = text.lines_at(pos.line as usize + 1);

        let mut before_line = before_iter.next();
        info!("before_line {:?}", before_line);

        let cursor_line1: String;
        let col = pos.character as usize;
        cursor_line1 = text.line(pos.line as usize).slice(0..col).to_string();
        // UNFINISHED LI|
        info!("cursor_line1 {:?}", cursor_line1);

        let mut after_line = after_iter.next();
        info!("after_line {:?}", after_line);

        let cursor_line2: String;
        if self.post.inputs.multiline {
            cursor_line2 = text.line(pos.line as usize).slice(col..).to_string();
        } else {
            cursor_line2 = "".to_string();
        }
        info!("cursor_line2 {:?}", cursor_line2);

        let mut before = vec![];
        let mut after = String::new();
        let mut stat_tokens = 0;
        while before_line.is_some() || after_line.is_some() {
            if let Some(before_line) = before_line {
                let before_line = before_line.to_string();
                let tokens = self.tokenizer.read().unwrap()
                    .encode(before_line.clone(), false)
                    .map_err(|err| {
                        return format!("Encoding error: {}", err);
                    })
                    .unwrap()
                    .len();
                if tokens > token_count {
                    break;
                }
                token_count -= tokens;
                stat_tokens += tokens;
                before.push(before_line);
            }
            if let Some(after_line) = after_line {
                let after_line = after_line.to_string();
                let tokens = self.tokenizer.read().unwrap()
                    .encode(after_line.clone(), false)
                    .map_err(|err| {
                        return format!("Encoding error: {}", err);
                    })
                   .unwrap()
                   .len();
                if tokens > token_count {
                    break;
                }
                token_count -= tokens;
                stat_tokens += tokens;
                after.push_str(&after_line);
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }
        info!("single file FIM prompt {} tokens < context {}", stat_tokens, context_size);
        let prompt = format!(
            "{}{}{}{}{}{}{}",
            fim_prefix,
            before.into_iter().rev().collect::<Vec<_>>().join(""),
            cursor_line1,
            fim_suffix,
            cursor_line2,
            after,
            fim_middle
        );
        if DEBUG {
            info!("prompt\n{}", prompt);
        }
        Ok(prompt)
    }

    fn re_stream_response(
        &self,
        model_says: serde_json::Value,
    ) -> Result<(serde_json::Value, bool), String> {
        if DEBUG {
            info!("re_stream_response\n{:?}\n", model_says);
        }
        let ans: serde_json::Value;
        let mut finish = false;

        if let Some(token) = model_says.get("token") {
            // streaming branch
            let mut token_text = "".to_string();
            if let Some(t) = token.get("text") {
                token_text = t.as_str().unwrap().to_string();
            }
            if token_text.contains("\n\n") || (token_text.contains("\n") && !self.post.inputs.multiline) {
                ans = serde_json::json!({
                    "code_completion_delta": cut_result(&token_text, "\n\n", self.post.inputs.multiline)
                });
                finish = true;
            } else {
                ans = serde_json::json!({
                    "code_completion_delta": token_text
                });
            }

        } else if let Some(arr) = model_says.as_array() {
            let tmp = arr.iter()
               .map(|x| {
                    let generated_text = x.get("generated_text").unwrap().as_str().unwrap();
                    serde_json::json!({
                        "code_completion": cut_result(&generated_text, "<|endoftext|>", self.post.inputs.multiline),
                    })
               }).collect::<Vec<_>>();
            ans = serde_json::json!(tmp);
            finish = true;

        } else {
            error!("No token or array {:?}", model_says);
            return Err("HF-style endpoint response unrecognized, see logs".to_string());
        }

        return Ok((ans, finish));
    }

}

fn cut_result(text: &str, eot_token: &str, multiline: bool) -> String {
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
        return text.to_string();
    }
    let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    // info!("cut_result text: {:?}, cut_at={:?}", text, cut_at);
    let ans = text.split_at(cut_at).0.to_string();
    ans.replace("\r", "")
}
