use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokenizers::Tokenizer;
use async_trait::async_trait;
use serde_json::Value;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::SamplingParameters;
use crate::tokens::count_text_tokens;

use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinishReason {
    None,
    Stop,
    Length,
    ScratchpadStop,
}

impl FinishReason {
    pub fn from_str(s: &str) -> FinishReason {
        match s {
            "" => FinishReason::None,
            "stop" => FinishReason::Stop,
            "stop-eot" => FinishReason::Stop,
            "stop-lf" => FinishReason::Stop,
            "tool_calls" => FinishReason::Stop,
            "length" => FinishReason::Length,
            "scratchpad-stop" => FinishReason::ScratchpadStop,
            _ => {
                warn!("Unknown finish reason: {}, interpreting it as a stop", s);
                FinishReason::Stop
            }
        }
    }

    pub fn from_json_val(json: &Value) -> Result<FinishReason, String> {
        if json.is_null() {
            return Ok(FinishReason::None);
        }
        if let Some(val) = json.as_str() {
            Ok(FinishReason::from_str(val))
        } else {
            Err(format!("expected string, got {}", json))
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            FinishReason::None => "".to_string(),
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "length".to_string(),
            // track this reason only inside the refact-lsp
            FinishReason::ScratchpadStop => "stop".to_string(),
        }
    }

    pub fn to_json_val(&self) -> Value {
        match self {
            FinishReason::None => Value::Null,
            _ => Value::String(self.to_string()),
        }
    }

    pub fn is_finished(&self) -> bool {
        self != &FinishReason::None
    }
}

#[async_trait]
pub trait ScratchpadAbstract: Send {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String>;

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String>;

    // Not streaming, convert what model says (choices) to final result
    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String>;

    // Only 1 choice, but streaming. Returns delta the user should see, and finished flag
    fn response_streaming(
        &mut self,
        delta: String,
        finish_reason: FinishReason
    ) -> Result<(Value, FinishReason), String>;

    fn response_message_n_choices(
        &mut self,
        _choices: Vec<String>,    // XXX replace with Value
        _finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        Err("not implemented".to_string())
    }

    fn response_message_streaming(
        &mut self,
        delta: &Value,
        finish_reason: FinishReason
    ) -> Result<(Value, FinishReason), String>;

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>;

    fn streaming_finished(&mut self, finish_reason: FinishReason) -> Result<Value, String>;
}

// aggregate this struct to make scratchpad implementation easier
#[derive(Debug, Clone)]
pub struct HasTokenizerAndEot {
    pub tokenizer: Option<Arc<Tokenizer>>,
    pub eot: String,
    pub eos: String,
    pub context_format: String,
    pub rag_ratio: f64,
}

impl HasTokenizerAndEot {
    pub fn new(tokenizer: Option<Arc<Tokenizer>>) -> Self {
        HasTokenizerAndEot { tokenizer, eot: String::new(), eos: String::new(), context_format: String::new(), rag_ratio: 0.5}
    }

    pub fn count_tokens(
        &self,
        text: &str,
    ) -> Result<i32, String> {
        count_text_tokens(self.tokenizer.clone(), text).map(|t| t as i32)
    }

    pub fn assert_one_token(
        &self,
        text: &str
    ) -> Result<(), String> {
        if self.tokenizer.is_none() {
            return Err("assert_one_token: no tokenizer".to_string());
        }

        let token_count = count_text_tokens(self.tokenizer.clone(), text)?;

        if token_count != 1 {
            Err(format!("assert_one_token: expected 1 token for \"{text}\", got {token_count}"))
        } else {
            Ok(())
        }
    }
}
