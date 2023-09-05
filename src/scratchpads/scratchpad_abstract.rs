use serde_json;

pub trait CodeCompletionScratchpad: Send {
    fn prompt(
        &self,
        context_size: usize,
    ) -> Result<String, String>;

    fn re_stream_response(
        &self,
        model_says: serde_json::Value,
    ) -> Result<(serde_json::Value, bool), String>;
}

