use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Weak};
use std::sync::RwLock as StdRwLock;

use tokenizers::Tokenizer;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::ast::chunk_utils::get_chunks;
use crate::ast::structs::FileASTMarkup;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::skeletonizer::make_formatter;
use crate::ast::treesitter::structs::SymbolType;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::structs::SplitResult;

const DEBUG: bool = true;


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
        tokens_limit: usize,
    ) -> Result<Vec<SplitResult>, String> {
        assert!(doc.text.is_some());
        let path = doc.path.clone();
        let path_str = doc.path.to_str().unwrap();

        let mut parser = match get_ast_parser_by_filename(&path) {
            Ok(parser) => parser,
            Err(_e) => {
                // info!("cannot find a parser for {:?}, using simple file splitter: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e.message);
                return self.fallback_file_splitter.vectorization_split(&doc, tokenizer.clone(), tokens_limit).await;
            }
        };

        let mut guid_to_children: HashMap<Uuid, Vec<Uuid>> = Default::default();
        let mut symbols_struct: Vec<SymbolInformation> = Default::default();
        {
            let symbols = parser.parse(doc.text_as_string().unwrap().as_str(), &path);
            let _ = symbols.into_iter().for_each(|s| {
                let s = s.read();
                guid_to_children.insert(s.guid().clone(), s.childs_guid().clone());
                symbols_struct.push(s.symbol_info_struct());
            });
        }

        let ast_markup: FileASTMarkup = match crate::ast::ast_file_markup::lowlevel_file_markup(&doc, &symbols_struct) {
            Ok(x) => x,
            Err(e) => {
                info!("lowlevel_file_markup failed for {:?}, using simple file splitter: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e);
                return self.fallback_file_splitter.vectorization_split(&doc, tokenizer.clone(), tokens_limit).await;
            }
        };

        let mut files_markup: HashMap<String, Arc<crate::scratchpads::chat_utils_rag::File>> = HashMap::new();
        files_markup.insert(path_str.to_string(), Arc::new(crate::scratchpads::chat_utils_rag::File { markup: ast_markup.clone(), cpath: path.clone(), cpath_symmetry_breaker: 0.0 }));
        let guid_to_info: HashMap<Uuid, &SymbolInformation> = ast_markup.symbols_sorted_by_path_len.iter().map(|s| (s.guid.clone(), s)).collect();
        let guids: Vec<_> = guid_to_info.iter().map(|(s, _)| s.clone()).collect();

        let mut chunks: Vec<SplitResult> = Vec::new();
        for guid in guids {
            let symbol = guid_to_info.get(&guid).unwrap();
            let need_in_vecdb_at_all = match symbol.symbol_type {
                SymbolType::StructDeclaration | SymbolType::FunctionDeclaration | 
                SymbolType::TypeAlias | SymbolType::ClassFieldDeclaration => true,
                _ => false,
            };
            if !need_in_vecdb_at_all {
                continue;
            }
            let formatter = make_formatter(&symbol.language);
            if symbol.symbol_type == SymbolType::StructDeclaration {
                if let Some(children) = guid_to_children.get(&symbol.guid) {
                    if !children.is_empty() {
                        let skeleton_line = formatter.make_skeleton(&symbol, &guid_to_children, &guid_to_info);
                        let chunks_ = get_chunks(&skeleton_line, &symbol.file_path,
                                                 &symbol.symbol_path,
                                                 (symbol.full_range.start_point.row, symbol.full_range.end_point.row),
                                                 tokenizer.clone(), tokens_limit, 0, true);
                        chunks.extend(chunks_);
                    }
                }
            }

            let (declaration, top_bottom_rows) = formatter.get_declaration_with_comments(&symbol, &guid_to_children, &guid_to_info);
            if !declaration.is_empty() {
                let chunks_ = get_chunks(&declaration, &symbol.file_path,
                                         &symbol.symbol_path, top_bottom_rows, tokenizer.clone(), tokens_limit, 0, true);
                chunks.extend(chunks_);
            }
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
