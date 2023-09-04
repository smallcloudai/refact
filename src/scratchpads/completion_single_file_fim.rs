use crate::scratchpads::scratchpad_abstract::CodeCompletionScratchpad;
use crate::scratchpads::call_validation::CodeCompletionPost;

use tokenizers::Tokenizer;
use tracing::info;


pub struct SingleFileFIM<'a> {
    pub tokenizer: &'a Tokenizer,
    pub post: &'a CodeCompletionPost,
}

impl<'a> SingleFileFIM<'a> {
    pub fn new(
        tokenizer: &'a Tokenizer,
        post: &'a CodeCompletionPost,
    ) -> Self {
        SingleFileFIM { tokenizer, post }
    }
}

impl<'a> CodeCompletionScratchpad for SingleFileFIM<'a> {
    fn prompt(
        &self,
        context_size: i32,
    ) {
        info!("This method is overridden in the derived class T={}", context_size);
        info!("post {:?}", self.post);
        info!("1");
        let toks = self.tokenizer.encode("hello world".to_string(), false).unwrap();
        info!("2");
        info!("toks: {:?}", toks);
    }

    fn re_stream_response(&self)
    {
    }
}
