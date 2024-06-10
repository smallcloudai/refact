use std::sync::{Arc, Weak};
use std::collections::HashMap;
use std::io::Write;
use tokio::sync::RwLock;
use md5;
use tokenizers::Tokenizer;
use tracing::info;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use std::sync::RwLock as StdRwLock;
use crate::ast::treesitter::structs::SymbolType;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::structs::SplitResult;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

const DEBUG: bool = false;


pub struct AstBasedFileSplitter {
    // soft_window: usize,
    // hard_window: usize,
    fallback_file_splitter: FileSplitter,
}

impl AstBasedFileSplitter {
    pub fn new(window_size: usize, soft_limit: usize) -> Self {
        Self {
            // soft_window: window_size,
            // hard_window: window_size + soft_limit,
            fallback_file_splitter: FileSplitter::new(window_size, soft_limit),
        }
    }

    pub async fn vectorization_split(
        &self,
        doc: &Document,
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        _gcx_weak: Weak<RwLock<GlobalContext>>,
        tokens_limit: usize
    ) -> Result<Vec<SplitResult>, String> {
        // let doc = doc.clone();
        assert!(doc.text.is_some());
        let doc_text: String = doc.text_as_string().unwrap();
        let path = doc.path.clone();
        let path_str = doc.path.to_str().unwrap();

        let mut parser = match get_ast_parser_by_filename(&path) {
            Ok(parser) => parser,
            Err(_e) => {
                // info!("cannot find a parser for {:?}, using simple file splitter: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e.message);
                return self.fallback_file_splitter.vectorization_split(&doc).await;
            }
        };

        let symbols_struct = parser.parse(doc.text_as_string().unwrap().as_str(), &path)
            .iter().map(|s| s.read().symbol_info_struct())
            .collect::<Vec<_>>();
        let ast_markup: crate::ast::structs::FileASTMarkup = match crate::ast::ast_file_markup::lowlevel_file_markup(&doc, &symbols_struct) {
            Ok(x) => x,
            Err(e) => {
                info!("lowlevel_file_markup failed for {:?}, using simple file splitter: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e);
                return self.fallback_file_splitter.vectorization_split(&doc).await;
            }
        };

        let mut files_markup: HashMap<String, Arc<crate::scratchpads::chat_utils_rag::File>> = HashMap::new();
        files_markup.insert(path_str.to_string(), Arc::new(crate::scratchpads::chat_utils_rag::File { markup: ast_markup.clone(), cpath: path.clone(), cpath_symmetry_breaker: 0.0 }));

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

        let mut chunks: Vec<SplitResult> = Vec::new();
        for symbol in ast_markup.symbols_sorted_by_path_len {
            let need_in_vecdb_at_all = match symbol.symbol_type {
                SymbolType::StructDeclaration | SymbolType::FunctionDeclaration | SymbolType::TypeAlias => true,
                _ => false,
            };
            if !need_in_vecdb_at_all {
                continue;
            }
            let text_short = symbol.shortened_text;
            let text_row1 = symbol.full_range.start_point.row;
            let text_row2 = symbol.full_range.end_point.row;
            let text_orig = doc_text.split("\n").skip(text_row1).take(text_row2 - text_row1 + 1).collect::<Vec<_>>().join("\n");
            // let text_short_tok_n = count_tokens(tokenizer.clone(), text_short.as_str());
            let text_orig_tok_n = count_tokens(tokenizer.clone(), text_orig.as_str());

            let (symbol_str, shortened) = if text_orig_tok_n < tokens_limit {
                (text_orig.clone(), false)
            } else {
                (text_short.clone(), true)
            };
            let symbol_lines = symbol_str.split("\n").collect::<Vec<_>>();

            let mut accum: String = String::new();
            let mut lines_so_far: usize = 0;
            let mut tokens_so_far: usize = 0;

            let mut flush_accum = |i: usize, lines_so_far: usize, accum: &String| {
                let (row1, row2) = if shortened {
                    (symbol.full_range.start_point.row, symbol.full_range.end_point.row)
                } else {
                    (symbol.full_range.start_point.row + i - lines_so_far, symbol.full_range.end_point.row + i)
                };
                chunks.push(SplitResult {
                    file_path: path.clone(),
                    window_text: accum.clone(),
                    window_text_hash: str_hash(&accum),
                    start_line: row1 as u64,
                    end_line: row2 as u64,
                    symbol_path: symbol.symbol_path.clone(),
                });
            };

            for (line_i, line) in symbol_lines.clone().into_iter().enumerate() {
                let tok_n: usize = count_tokens(tokenizer.clone(), line);
                if tokens_so_far + tok_n > tokens_limit {
                    flush_accum(line_i, lines_so_far, &accum);
                    accum.clear();
                    tokens_so_far = 0;
                    lines_so_far = 0;
                }
                accum.push_str(line);
                accum.push('\n');
                tokens_so_far += tok_n;
                lines_so_far += 1;
            }
            flush_accum(symbol_lines.len(), lines_so_far, &accum);
        }
        let path_vecdb = path.with_extension("vecdb");
        if DEBUG {
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
