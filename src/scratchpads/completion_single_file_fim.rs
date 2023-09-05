use crate::scratchpads::scratchpad_abstract::CodeCompletionScratchpad;
use crate::scratchpads::call_validation::CodeCompletionPost;
use std::sync::Arc;
use std::sync::RwLock;

use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::info;


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
        let mut before_iter = text.lines_at(pos.line as usize + 1).reversed();
        let mut after_iter = text.lines_at(pos.line as usize);
        let mut before_line = before_iter.next();
        let col = pos.character as usize;
        if let Some(line) = before_line {
            before_line = Some(line.slice(0..col));
        }
        let mut after_line = after_iter.next();
        if let Some(line) = after_line {
            after_line = Some(line.slice(col..));
        }
        let mut before = vec![];
        let mut after = String::new();
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
                after.push_str(&after_line);
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }
        Ok(format!(
            "{}{}{}{}{}",
            fim_prefix,
            before.into_iter().rev().collect::<Vec<_>>().join(""),
            fim_suffix,
            after,
            fim_middle
        ))
    }

    fn re_stream_response(
        &self,
        model_says: serde_json::Value,
    ) -> Result<(serde_json::Value, bool), String> {
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
            return Err("No token or array".to_string());
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
    info!("cut_result text: {:?}, cut_at={:?}", text, cut_at);
    text.split_at(cut_at).0.to_string()
}
