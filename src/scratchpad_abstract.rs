use serde_json;
use std::sync::Arc;
use std::sync::RwLock;
use tokenizers::Tokenizer;
use crate::call_validation::SamplingParameters;


pub trait CodeCompletionScratchpad: Send {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String>;

    fn prompt(
        &self,
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String>;

    fn re_stream_response(
        &self,
        model_says: serde_json::Value,
    ) -> Result<(serde_json::Value, bool), String>;
}


// aggregate this struct to make scratchpad implementation easier
#[derive(Debug, Clone)]
pub struct HasTokenizerAndEot {
    pub tokenizer: Arc<RwLock<Tokenizer>>,
    pub eot: String,
}

impl HasTokenizerAndEot {
    pub fn new(tokenizer: Arc<RwLock<Tokenizer>>) -> Self {
        HasTokenizerAndEot { tokenizer, eot: String::new() }
    }

    pub fn count_tokens(
        &self,
        text: &str,
    ) -> Result<usize, String> {
        let tokenizer = self.tokenizer.write().unwrap();
        let tokens = tokenizer.encode(text, false).map_err(|err| {
            return format!("Encoding error: {}", err);
        })?;
        Ok(tokens.len())
    }

    pub fn assert_one_token(
        &self,
        text: &str
    ) -> Result<(), String> {
        let tokenizer = self.tokenizer.write().unwrap();
        let tokens = tokenizer.encode(text, false).map_err(|err| {
            format!("assert_one_token: {}", err)
        })?;
        if tokens.len() != 1 {
            return Err(format!("assert_one_token: expected 1 token for \"{}\", got {}", text, tokens.len()));
        }
        Ok(())
    }
}
