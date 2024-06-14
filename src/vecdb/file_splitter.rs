use std::io::Write;
use std::sync::Arc;
use md5;
use tokenizers::Tokenizer;
use std::sync::RwLock as StdRwLock;
use crate::ast::chunk_utils::get_chunks;
use crate::ast::count_tokens;
use crate::ast::file_splitter::INTERSECTION_LINES;
use crate::vecdb::structs::SplitResult;
use crate::files_in_workspace::Document;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub struct FileSplitter {
    soft_window: usize,
}


impl FileSplitter {
    pub fn new(window_size: usize) -> Self {
        Self {
            soft_window: window_size,
        }
    }

    pub async fn vectorization_split(&self, doc: &Document,
                                     tokenizer: Arc<StdRwLock<Tokenizer>>,
                                     tokens_limit: usize,
    ) -> Result<Vec<SplitResult>, String> {
        let path = doc.path.clone();
        let text = match doc.clone().get_text_or_read_from_disk().await {
            Ok(s) => s,
            Err(e) => return Err(e.to_string())
        };

        let mut chunks = Vec::new();

        let mut lines_accumulator: Vec<&str> = Default::default();
        let mut token_n_accumulator = 0;
        let mut top_row: i32 = -1;
        let lines = text.split('\n').collect::<Vec<_>>();
        for (line_idx, line) in lines.iter().enumerate() {
            let text_orig_tok_n = count_tokens(tokenizer.clone(), line);
            if top_row == -1 && text_orig_tok_n != 0 { // top lines are empty
                top_row = line_idx as i32;
            }
            if top_row == -1 { // skip empty lines, if accums are empty
                continue;
            }
            if token_n_accumulator + text_orig_tok_n < self.soft_window {
                lines_accumulator.push(line);
                token_n_accumulator += text_orig_tok_n;
                continue;
            }

            if line.is_empty() { // end of paragraph
                let _line = lines_accumulator.join("\n");
                let chunks_ = get_chunks(&_line, &path, &"".to_string(),
                                         (top_row as usize, line_idx - 1),
                                         tokenizer.clone(), tokens_limit, INTERSECTION_LINES, false);
                chunks.extend(chunks_);
                lines_accumulator.clear();
                token_n_accumulator = 0;
                top_row = -1;
            } else {
                lines_accumulator.push(line);
                token_n_accumulator += text_orig_tok_n;
            }
        }
        if !lines_accumulator.is_empty() {
            let _line = lines_accumulator.join("\n");
            let chunks_ = get_chunks(&_line, &path, &"".to_string(),
                                     (top_row as usize, lines.len() - 1),
                                     tokenizer.clone(), tokens_limit, INTERSECTION_LINES, false);
            chunks.extend(chunks_);
        }
        
        if crate::ast::file_splitter::DEBUG {
            let path_vecdb = path.with_extension("vecdb");
            if let Ok(mut file) = std::fs::File::create(path_vecdb) {
                let mut writer = std::io::BufWriter::new(&mut file);
                for chunk in chunks.iter() {
                    let beautiful_line = format!("\n\n------- {}:{}-{} ------\n", chunk.symbol_path, chunk.start_line, chunk.end_line);
                    let _ = writer.write_all(beautiful_line.as_bytes());
                    let _ = writer.write_all(chunk.window_text.as_bytes());
                    let _ = writer.write_all(b"\n");
                }
            }
        }
        Ok(chunks)
    }
}
