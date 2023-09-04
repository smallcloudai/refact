// mod scratchpads {
pub mod completion_single_file_fim;
pub mod scratchpad_abstract;
pub mod call_validation;
use tokenizers::Tokenizer;
use crate::scratchpads::call_validation::CodeCompletionPost;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;


pub fn create_code_completion_scratchpad(
    t: Arc<StdRwLock<Tokenizer>>,
    post: CodeCompletionPost,
) -> Box<dyn scratchpad_abstract::CodeCompletionScratchpad> {
    // FIXME: pick scratchpad depending on code_completion_post.model
    Box::new(completion_single_file_fim::SingleFileFIM::new(t, post))
}
