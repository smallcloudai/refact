use std::sync::Arc;
use md5;
use tokenizers::Tokenizer;
use std::sync::RwLock as StdRwLock;
use crate::ast::count_tokens;
use crate::vecdb::structs::SplitResult;
use crate::files_in_workspace::Document;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub struct FileSplitter {
    soft_window: usize,
    hard_window: usize,
}

impl FileSplitter {
    pub fn new(window_size: usize, soft_limit: usize) -> Self {
        Self {
            soft_window: window_size,
            hard_window: window_size + soft_limit,
        }
    }

    pub async fn vectorization_split(&self, doc: &Document,
                                     tokenizer: Arc<StdRwLock<Tokenizer>>,
                                     tokens_limit: usize,
                                    ) -> Result<Vec<SplitResult>, String> {
        let text = match doc.clone().get_text_or_read_from_disk().await {
            Ok(s) => s,
            Err(e) => return Err(e.to_string())
        };

        let mut chunks = Vec::new();
        let mut current_line = String::default();
        let mut current_token_n = 0;

        let mut current_line_number: u64 = 0;
        let mut idx_last_line: u64 = 0;
        for (line_idx, line) in text.lines().enumerate() {
            let line_with_newline = if current_line.is_empty() { line.to_string() } else { format!("\n{}", line) };
            let text_orig_tok_n = count_tokens(tokenizer.clone(), line_with_newline.as_str());
            if current_token_n + text_orig_tok_n > tokens_limit {
                chunks.push(SplitResult {
                    file_path: doc.path.clone(),
                    window_text: current_line.clone(),
                    window_text_hash: str_hash(&current_line),
                    start_line: current_line_number,
                    end_line: (line_idx - 1) as u64,
                    symbol_path: "".to_string(),
                });
                current_line = line.to_string();
                current_token_n = text_orig_tok_n;
                current_line_number = line_idx as u64;
            } else {
                current_token_n += text_orig_tok_n;
                current_line = format!("{}{}", current_line, line_with_newline);
            }
            idx_last_line = line_idx as u64;
        }

        if !current_line.is_empty() {
            chunks.push(SplitResult {
                file_path: doc.path.clone(),
                window_text: current_line.clone(),
                window_text_hash: str_hash(&current_line),
                start_line: current_line_number,
                end_line: idx_last_line,
                symbol_path: "".to_string(),
            });
        }

        Ok(chunks)
    }
}
