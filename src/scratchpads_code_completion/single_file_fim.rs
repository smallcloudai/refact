use crate::scratchpad_abstract::Scratchpad;
use tokenizers::Tokenizer;
use reqwest::header::AUTHORIZATION;
use tracing::{error, info};


pub struct SingleFileFIM {
    pub tokenizer: Tokenizer,
    pub model: String,
}

impl SingleFileFIM {
    pub fn new(
        tokenizer: &Tokenizer,
        model: &str,
    ) -> Self {
        Self {
            tokenizer: tokenizer.clone(),
            model: model.to_string()
        }
    }
}

impl Scratchpad for SingleFileFIM {
    fn prompt(
        &self,
        context_size: usize,
    ) {
        println!("This method is overridden in the derived class T={}", context_size);
        info!("1");
        let toks = self.tokenizer.encode("hello world".to_string(), false).unwrap();
        info!("2");
        info!("toks: {:?}", toks);
    }

    fn re_stream_response()
    {
    }
}
