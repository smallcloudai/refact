use serde_json::Value;
use tokenizers::Tokenizer;

use crate::postprocessing::pp_context_files::RESERVE_FOR_QUESTION_AND_FOLLOWUP;


pub struct HasRagResults {
    pub was_sent: bool,
    pub in_json: Vec<Value>,
}

impl HasRagResults {
    pub fn new() -> Self {
        HasRagResults {
            was_sent: false,
            in_json: vec![],
        }
    }
}

impl HasRagResults {
    pub fn push_in_json(&mut self, value: Value) {
        self.in_json.push(value);
    }

    pub fn response_streaming(&mut self) -> Result<Vec<Value>, String> {
        if self.was_sent == true || self.in_json.is_empty() {
            return Ok(vec![]);
        }
        self.was_sent = true;
        Ok(self.in_json.clone())
    }
}

pub fn count_tokens(
    tokenizer: &Tokenizer,
    content: &crate::call_validation::ChatContent,
) -> usize {
    // XXX count image size
    count_tokens_text_only(tokenizer, content.content_text_only().as_str())
}

pub fn count_tokens_text_only(
    tokenizer: &Tokenizer,
    text: &str,
) -> usize {
    match tokenizer.encode(text, false) {
        Ok(tokens) => tokens.len(),
        Err(_) => 0,
    }
}

pub fn max_tokens_for_rag_chat(n_ctx: usize, maxgen: usize) -> usize {
    (n_ctx/2).saturating_sub(maxgen).saturating_sub(RESERVE_FOR_QUESTION_AND_FOLLOWUP)
}

