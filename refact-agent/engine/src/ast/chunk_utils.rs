use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use itertools::Itertools;
use ropey::Rope;
use tokenizers::Tokenizer;

use crate::ast::count_tokens;
use crate::vecdb::vdb_structs::SplitResult;


pub fn official_text_hashing_function(s: &str) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}


fn split_line_if_needed(line: &str, tokenizer: Option<Arc<StdRwLock<Tokenizer>>>, tokens_limit: usize) -> Vec<String> {
    if let Some(tokenizer) = tokenizer {
        let tokenizer = tokenizer.read().unwrap();
        tokenizer.encode(line, false).map_or_else(
            |_| split_without_tokenizer(line, tokens_limit),
            |tokens| {
                let ids = tokens.get_ids();
                if ids.len() <= tokens_limit {
                    vec![line.to_string()]
                } else {
                    ids.chunks(tokens_limit)
                        .filter_map(|chunk| tokenizer.decode(chunk, true).ok())
                        .collect()
                }
            }
        )
    } else {
        split_without_tokenizer(line, tokens_limit)
    }
}

fn split_without_tokenizer(line: &str, tokens_limit: usize) -> Vec<String> {
    if count_tokens(None, line) <= tokens_limit {
        vec![line.to_string()]
    } else {
        Rope::from_str(line).chars()
            .collect::<Vec<_>>()
            .chunks(tokens_limit)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }
}

pub fn get_chunks(text: &String,
                  file_path: &PathBuf,
                  symbol_path: &String,
                  top_bottom_rows: (usize, usize), // case with top comments
                  tokenizer: Option<Arc<StdRwLock<Tokenizer>>>,
                  tokens_limit: usize,
                  intersection_lines: usize,
                  use_symbol_range_always: bool, // use for skeleton case
) -> Vec<SplitResult> {
    let (top_row, bottom_row) = top_bottom_rows;
    let mut chunks: Vec<SplitResult> = Vec::new();
    let mut accum: VecDeque<(String, usize)> = Default::default();
    let mut current_tok_n = 0;
    let lines = text.split("\n").collect::<Vec<&str>>();

    {  // try to split chunks from top to bottom
        let mut line_idx: usize = 0;
        let mut previous_start = line_idx;
        while line_idx < lines.len() {
            let line = lines[line_idx];
            let line_tok_n = count_tokens(tokenizer.clone(), line);

            if !accum.is_empty() && current_tok_n + line_tok_n > tokens_limit {
                let current_line = accum.iter().map(|(line, _)| line).join("\n");
                let start_line = if use_symbol_range_always { top_row as u64 } else { accum.front().unwrap().1 as u64 };
                let end_line = if use_symbol_range_always { bottom_row as u64 } else { accum.back().unwrap().1 as u64 };
                for chunked_line in split_line_if_needed(&current_line, tokenizer.clone(), tokens_limit) {
                    chunks.push(SplitResult {
                        file_path: file_path.clone(),
                        window_text: chunked_line.clone(),
                        window_text_hash: official_text_hashing_function(&chunked_line),
                        start_line,
                        end_line,
                        symbol_path: symbol_path.clone(),
                    });
                }
                accum.clear();
                current_tok_n = 0;
                line_idx = (previous_start + 1).max((line_idx as i64 - intersection_lines as i64).max(0) as usize);
                previous_start = line_idx;
            } else {
                current_tok_n += line_tok_n;
                accum.push_back((line.to_string(), line_idx + top_row));
                line_idx += 1;
            }
        }
    }

    // fill last chunk from bottom up to tokens_limit
    if !accum.is_empty() {
        let mut line_idx: i64 = (lines.len() - 1) as i64;
        accum.clear();
        current_tok_n = 0;
        while line_idx >= 0 {
            let line = lines[line_idx as usize];
            let text_orig_tok_n = count_tokens(tokenizer.clone(), line);
            if !accum.is_empty() && current_tok_n + text_orig_tok_n > tokens_limit {
                let current_line = accum.iter().map(|(line, _)| line).join("\n");
                let start_line = if use_symbol_range_always { top_row as u64 } else { accum.front().unwrap().1 as u64 };
                let end_line = if use_symbol_range_always { bottom_row as u64 } else { accum.back().unwrap().1 as u64 };
                for chunked_line in split_line_if_needed(&current_line, tokenizer.clone(), tokens_limit) {
                    chunks.push(SplitResult {
                        file_path: file_path.clone(),
                        window_text: chunked_line.clone(),
                        window_text_hash: official_text_hashing_function(&chunked_line),
                        start_line,
                        end_line,
                        symbol_path: symbol_path.clone(),
                    });
                }
                accum.clear();
                break;
            } else {
                current_tok_n += text_orig_tok_n;
                accum.push_front((line.to_string(), line_idx as usize + top_row));
                line_idx -= 1;
            }
        }
    }

    if !accum.is_empty() {
        let current_line = accum.iter().map(|(line, _)| line).join("\n");
        let start_line = if use_symbol_range_always { top_row as u64 } else { accum.front().unwrap().1 as u64 };
        let end_line = if use_symbol_range_always { bottom_row as u64 } else { accum.back().unwrap().1 as u64 };
        for chunked_line in split_line_if_needed(&current_line, tokenizer.clone(), tokens_limit) {
            chunks.push(SplitResult {
                file_path: file_path.clone(),
                window_text: chunked_line.clone(),
                window_text_hash: official_text_hashing_function(&chunked_line),
                start_line,
                end_line,
                symbol_path: symbol_path.clone(),
            });
        }
    }

    chunks.into_iter().filter(|c|!c.window_text.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock as StdRwLock};

    use crate::ast::chunk_utils::get_chunks;
    use crate::ast::count_tokens;
    // use crate::vecdb::vdb_structs::SplitResult;

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

    #[test]
    fn dummy_tokenizer_test() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let text_orig_tok_n = count_tokens(Some(tokenizer.clone()), PYTHON_CODE);
        assert_eq!(text_orig_tok_n, PYTHON_CODE.len());
    }

    #[test]
    fn simple_chunk_test_1_with_128_limit() {
        let tokenizer = Arc::new(StdRwLock::new(tokenizers::Tokenizer::from_str(DUMMY_TOKENIZER).unwrap()));
        let orig = include_str!("../caps.rs").to_string();
        let token_limits = [10, 50, 100, 200, 300];
        for &token_limit in &token_limits {
            let chunks = get_chunks(
                &orig,
                &PathBuf::from_str("/tmp/test.py").unwrap(),
                &"".to_string(),
                (0, 10),
                Some(tokenizer.clone()),
                token_limit, 2, false);
            let mut not_present: Vec<char> = orig.chars().collect();
            let mut result = String::new();
            for chunk in chunks.iter() {
                result.push_str(&format!("\n\n------- {:?} {}-{} -------\n", chunk.symbol_path, chunk.start_line, chunk.end_line));
                result.push_str(&chunk.window_text);
                result.push_str("\n");
                let mut start_pos = 0;
                while let Some(found_pos) = orig[start_pos..].find(&chunk.window_text) {
                    let i = start_pos + found_pos;
                    for j in i .. i + chunk.window_text.len() {
                        not_present[j] = ' ';
                    }
                    start_pos = i + chunk.window_text.len();
                }
            }
            let not_present_str = not_present.iter().collect::<String>();
            println!("====\n{}\n====", result);
            assert!(not_present_str.trim().is_empty(), "token_limit={} anything non space means it's missing from vecdb {:?}", token_limit, not_present_str);
        }
    }

}
