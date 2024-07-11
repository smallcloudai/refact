use std::cmp::max;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use itertools::Itertools;
use tokenizers::Tokenizer;

use crate::ast::count_tokens;
use crate::vecdb::vdb_structs::SplitResult;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub fn get_chunks(text: &String,
                  file_path: &PathBuf,
                  symbol_path: &String,
                  top_bottom_rows: (usize, usize), // case with top comments
                  tokenizer: Arc<StdRwLock<Tokenizer>>,
                  tokens_limit: usize,
                  intersection_lines: usize,
                  use_symbol_range_always: bool, // use for skeleton case
) -> Vec<SplitResult> {
    let (top_row, bottom_row) = top_bottom_rows;
    let mut chunks: Vec<SplitResult> = Vec::new();
    let mut current_line_accum: VecDeque<&str> = Default::default();
    let mut current_line_number: u64 = top_row as u64;
    let mut current_token_n = 0;
    let lines = text.split("\n").collect::<Vec<&str>>();

    {  // try to split chunks from top to bottom
        let mut line_idx: usize = 0;
        while line_idx < lines.len() {
            let line = lines[line_idx];
            let line_with_newline = if current_line_accum.is_empty() { line.to_string() } else { format!("{}\n", line) };
            let text_orig_tok_n = count_tokens(tokenizer.clone(), line_with_newline.as_str());

            if current_token_n + text_orig_tok_n > tokens_limit {
                let current_line = current_line_accum.iter().join("\n");
                chunks.push(SplitResult {
                    file_path: file_path.clone(),
                    window_text: current_line.clone(),
                    window_text_hash: str_hash(&current_line),
                    start_line: if use_symbol_range_always { top_row as u64 } else { current_line_number },
                    end_line: if use_symbol_range_always { bottom_row as u64 } else { max(top_row as i64, top_row as i64 + line_idx as i64 - 1) as u64 },
                    symbol_path: symbol_path.clone(),
                });
                current_line_accum.clear();
                current_token_n = 0;
                current_line_number = current_line_number + line_idx as u64 - intersection_lines as u64;
                line_idx -= intersection_lines;
            } else {
                current_token_n += text_orig_tok_n;
                current_line_accum.push_back(line);
                line_idx += 1;
            }
        }
    }

    if !current_line_accum.is_empty() {  // try to fill last chunk from bottom in tokens_limit
        let mut line_idx: i64 = (lines.len() - 1) as i64;
        current_line_accum.clear();
        current_token_n = 0;
        while line_idx >= 0 {
            let line = lines[line_idx as usize];
            let line_with_newline = if current_line_accum.is_empty() { line.to_string() } else { format!("{}\n", line) };
            let text_orig_tok_n = count_tokens(tokenizer.clone(), line_with_newline.as_str());
            if current_token_n + text_orig_tok_n > tokens_limit {
                let current_line = current_line_accum.iter().join("\n");
                chunks.push(SplitResult {
                    file_path: file_path.clone(),
                    window_text: current_line.clone(),
                    window_text_hash: str_hash(&current_line),
                    start_line: if use_symbol_range_always { top_row as u64 } else { top_row as u64 + line_idx as u64 + 1 },
                    end_line: if use_symbol_range_always { bottom_row as u64 } else { bottom_row as u64 },
                    symbol_path: symbol_path.clone(),
                });
                current_line_accum.clear();
                break;
            } else {
                current_token_n += text_orig_tok_n;
                current_line_accum.push_front(line);
                line_idx -= 1;
            }
        }
    }
    // flush accumulators
    if !current_line_accum.is_empty() {
        let current_line = current_line_accum.iter().join("\n");
        chunks.push(SplitResult {
            file_path: file_path.clone(),
            window_text: current_line.clone(),
            window_text_hash: str_hash(&current_line),
            start_line: top_row as u64,
            end_line: bottom_row as u64,
            symbol_path: symbol_path.clone(),
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock as StdRwLock};

    use crate::ast::chunk_utils::get_chunks;
    use crate::ast::count_tokens;
    use crate::vecdb::vdb_structs::SplitResult;

    const DUMMY_TOKENIZER: &str = include_str!("dummy_tokenizer.json");
    const PYTHON_CODE: &str = r#"def square_number(x):
    """
    This function takes a number and returns its square.

    Parameters:
    x (int): A number to be squared.

    Returns:
    int: The square of the input number.
    """
    return x**2"#;

    static FULL_CHUNK_RANGES_128: [(usize, usize); 2] = [(0, 4), (5, 10)];
    static FULL_CHUNK_WITH_STRIDE_2_RANGES_128: [(usize, usize); 3] = [(0, 4), (3, 9), (5, 10)];
    static SKIP_0_LINE_CHUNK_RANGES_128: [(usize, usize); 3] = [(1, 4), (5, 9), (6, 10)];
    static FULL_CHUNK_RANGES_100: [(usize, usize); 3] = [(0, 3), (4, 7), (6, 10)];
    static TAKE_SMALL_CHUNK_RANGES_128: [(usize, usize); 1] = [(0, 3)];

    fn base_check_chunks(text: &str, chunks: Vec<SplitResult>, ref_data: &[(usize, usize)]) {
        let lines = text.lines().collect::<Vec<&str>>();
        assert_eq!(chunks.len(), ref_data.len());
        for (idx, range) in ref_data.iter().enumerate() {
            assert_eq!(chunks[idx].start_line, range.0 as u64);
            assert_eq!(chunks[idx].end_line, range.1 as u64);
            let ref_content = lines[range.0..range.1 + 1].join("\n");
            assert_eq!(chunks[idx].window_text, ref_content);
        }
    }

    #[test]
    fn dummy_tokenizer_test() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let text_orig_tok_n = count_tokens(tokenizer.clone(), PYTHON_CODE);
        assert_eq!(text_orig_tok_n, PYTHON_CODE.len());
    }

    #[test]
    fn simple_chunk_test_1_with_128_limit() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let chunks = get_chunks(&PYTHON_CODE.to_string(),
                                &PathBuf::from_str("/tmp/test.py").unwrap(),
                                &"".to_string(),
                                (0, 10),
                                tokenizer.clone(),
                                128, 0, false);
        base_check_chunks(&PYTHON_CODE, chunks, &FULL_CHUNK_RANGES_128);
    }

    #[test]
    fn simple_chunk_test_with_100_limit() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let chunks = get_chunks(&PYTHON_CODE.to_string(),
                                &PathBuf::from_str("/tmp/test.py").unwrap(),
                                &"".to_string(),
                                (0, 10),
                                tokenizer.clone(),
                                100, 0, false);
        base_check_chunks(&PYTHON_CODE, chunks, &FULL_CHUNK_RANGES_100);
    }

    #[test]
    fn simple_chunk_test_2_with_128_limit() {
        let content = {
            let lines = PYTHON_CODE.lines().collect::<Vec<&str>>();
            lines[1..lines.len()].join("\n")
        };

        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let chunks = get_chunks(&content.to_string(),
                                &PathBuf::from_str("/tmp/test.py").unwrap(),
                                &"".to_string(),
                                (1, 10),
                                tokenizer.clone(),
                                100, 0, false);
        base_check_chunks(&PYTHON_CODE, chunks, &SKIP_0_LINE_CHUNK_RANGES_128);
    }

    #[test]
    fn simple_chunk_test_3_with_128_limit() {
        let content = {
            let lines = PYTHON_CODE.lines().collect::<Vec<&str>>();
            lines[0..4].join("\n")
        };

        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let chunks = get_chunks(&content.to_string(),
                                &PathBuf::from_str("/tmp/test.py").unwrap(),
                                &"".to_string(),
                                (0, 3),
                                tokenizer.clone(),
                                100, 0, false);
        base_check_chunks(&PYTHON_CODE, chunks, &TAKE_SMALL_CHUNK_RANGES_128);
    }

    #[test]
    fn simple_chunk_test_with_stride_1_with_128_limit() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let chunks = get_chunks(&PYTHON_CODE.to_string(),
                                &PathBuf::from_str("/tmp/test.py").unwrap(),
                                &"".to_string(),
                                (0, 10),
                                tokenizer.clone(),
                                128, 2, false);
        base_check_chunks(&PYTHON_CODE, chunks, &FULL_CHUNK_WITH_STRIDE_2_RANGES_128);
    }
}