use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use tokenizers::Tokenizer;

pub mod treesitter;
pub mod ast_index;
pub mod ast_index_service;
pub mod ast_module;
pub mod ast_file_markup;
pub mod structs;
pub mod file_splitter;
pub mod comments_wrapper;
mod usages_declarations_merger;


mod imports_resolver;
pub mod chunk_utils;

pub fn count_tokens(
    tokenizer: Arc<StdRwLock<Tokenizer>>,
    text: &str,
) -> usize {
    let tokenizer_locked = tokenizer.write().unwrap();
    let tokens = match tokenizer_locked.encode(text, false) {
        Ok(tokens) => tokens,
        Err(err) => {
            tracing::warn!("Encoding error: {}", err);
            return 0;
        }
    };
    tokens.len()
}