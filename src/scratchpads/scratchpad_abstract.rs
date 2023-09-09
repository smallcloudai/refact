use crate::call_validation::SamplingParameters;
use serde_json;


pub trait CodeCompletionScratchpad: Send {
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

