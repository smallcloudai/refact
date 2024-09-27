use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;

pub mod ast_structs;
pub mod ast_db;
pub mod ast_parse_anything;

pub mod treesitter;
pub mod ast_indexer_thread;
pub mod file_splitter;
pub mod chunk_utils;
pub mod linters;


pub fn count_tokens(
    tokenizer: Option<Arc<StdRwLock<Tokenizer>>>,
    text: &str,
) -> usize {
    if let Some(tokenizer) = tokenizer {
        let tokenizer_locked = tokenizer.write().unwrap();
        let tokens = match tokenizer_locked.encode(text, false) {
            Ok(tokens) => tokens,
            Err(err) => {
                tracing::warn!("Encoding error: {}", err);
                return 0;
            }
        };
        tokens.len()
    } else {
        1 + text.len() / 3
    }
}
