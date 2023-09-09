pub mod completion_single_file_fim;
use tokenizers::Tokenizer;
use crate::call_validation::CodeCompletionPost;
use crate::scratchpad_abstract::CodeCompletionScratchpad;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;


fn verify_has_send<T: Send>(_x: &T) {}


pub fn create_code_completion_scratchpad(
    post: CodeCompletionPost,
    scratchpad_name: &str,
    scratchpad_patch: &serde_json::Value,
    tokenizer_arc: Arc<StdRwLock<Tokenizer>>,
) -> Result<Box<dyn CodeCompletionScratchpad>, String> {
    let mut result: Box<dyn CodeCompletionScratchpad>;
    if scratchpad_name == "FIM-PSM" {
        result = Box::new(completion_single_file_fim::SingleFileFIM::new(tokenizer_arc, post, "PSM".to_string()));
    } else if scratchpad_name == "FIM-SPM" {
        result = Box::new(completion_single_file_fim::SingleFileFIM::new(tokenizer_arc, post, "SPM".to_string()));
    } else {
        return Err(format!("This rust binary doesn't have scratchpad \"{}\" compiled in", scratchpad_name));
    }
    result.apply_model_adaptation_patch(scratchpad_patch)?;
    verify_has_send(&result);
    Ok(result)
}
