// mod scratchpads {
pub mod completion_single_file_fim;
pub mod scratchpad_abstract;
pub mod call_validation;
use tokenizers::Tokenizer;
use crate::scratchpads::call_validation::CodeCompletionPost;


pub fn create_code_completion_scratchpad<'a>(
    t: &'a Tokenizer,
    post: &'a CodeCompletionPost,
) -> Box<dyn scratchpad_abstract::CodeCompletionScratchpad + 'a> {
    // FIXME: pick scratchpad depending on code_completion_post.model
    Box::new(completion_single_file_fim::SingleFileFIM::new(t, post))
}
