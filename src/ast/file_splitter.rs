use std::sync::{Arc, Weak};
use std::collections::HashMap;
use tokio::sync::RwLock;
use md5;
use tokenizers::Tokenizer;
use tracing::info;
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use std::sync::RwLock as StdRwLock;
use crate::ast::treesitter::structs::SymbolType;
use crate::call_validation::ContextFile;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::structs::SplitResult;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

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
        global_context: Weak<RwLock<GlobalContext>>,
        tokens_limit: usize
    ) -> Result<Vec<SplitResult>, String> {
        // let doc = doc.clone();
        assert!(doc.text.is_some());
        let path = doc.path.clone();
        let path_str = doc.path.to_str().unwrap();

        let mut parser = match get_ast_parser_by_filename(&path) {
            Ok(parser) => parser,
            Err(e) => {
                info!("cannot find a parser for {:?}, using simple file splitter: {}", path, e.message);
                return self.fallback_file_splitter.split(&doc).await;
            }
        };

        let symbols = parser.parse(doc.text_as_string().unwrap().as_str(), &path);

        let ast_markup: crate::ast::structs::FileASTMarkup = match crate::ast::ast_file_markup::lowlevel_file_markup(&doc, &symbols).await {
            Ok(x) => x,
            Err(e) => {
                info!("lowlevel_file_markup failed for {:?}, using simple file splitter: {}", path, e);
                return self.fallback_file_splitter.split(&doc).await;
            }
        };
        let mut files_markup: HashMap<String, Arc<crate::scratchpads::chat_utils_rag::File>> = HashMap::new();
        files_markup.insert(path_str.to_string(), Arc::new(crate::scratchpads::chat_utils_rag::File { markup: ast_markup, cpath: path.clone(), cpath_symmetry_breaker: 0.0 }));

        let mut chunks: Vec<SplitResult> = Vec::new();
        // let mut split_normally: usize = 0;
        // let mut split_using_fallback: usize = 0;
        // let mut split_errors: usize = 0;
        for symbol in symbols.iter().map(|s| s.read()
            .expect("cannot read symbol")
            .symbol_info_struct())
        {
            // TypeAlias,
            // ClassFieldDeclaration,
            // ImportDeclaration,
            // VariableDefinition,
            // FunctionDeclaration,
            // CommentDefinition,
            // FunctionCall,
            // VariableUsage,
            let go_via_postprocessing = match symbol.symbol_type {
                SymbolType::StructDeclaration | SymbolType::FunctionDeclaration | SymbolType::TypeAlias => true,
                _ => false,
            };
            if go_via_postprocessing {
                let full_range = symbol.full_range;
                let messages = vec![ContextFile {
                    file_name: symbol.file_path.to_str().unwrap().parse().unwrap(),
                    file_content: "".to_string(),
                    line1: full_range.start_point.row + 1,
                    line2: full_range.end_point.row + 1,
                    symbol: symbol.guid.clone(),
                    gradient_type: -1,
                    usefulness: 100.0,
                }];
                let single_file_mode = true;

                let mut settings = crate::scratchpads::chat_utils_rag::PostprocessSettings::new();
                settings.take_floor = 50.0;
                settings.useful_background = 0.0;
                settings.useful_symbol_default = 0.0;
                settings.close_small_gaps = false;
                let (mut lines_in_files, mut lines_by_useful) = crate::scratchpads::chat_utils_rag::postprocess_rag_stage_3_6(
                    global_context.upgrade().unwrap(),
                    messages,
                    &files_markup,
                    &settings,
                ).await;

                let res = crate::scratchpads::chat_utils_rag::postprocess_rag_stage_7_9(
                    &mut lines_in_files,
                    &mut lines_by_useful,
                    tokenizer.clone(),
                    tokens_limit,
                    single_file_mode,
                    &settings,
                ).await;

                if let Some(first) = res.first() {
                    let mut content = first.file_content.clone();
                    if content.starts_with("...\n") {
                        content = content[4..].to_string();
                    }
                    info!("{:?} content updated:\n{}", symbol.name, content);
                    chunks.push(SplitResult {
                        file_path: path.clone(),
                        window_text: content.clone(),
                        window_text_hash: str_hash(&content),
                        start_line: symbol.full_range.start_point.row as u64,
                        end_line: symbol.full_range.end_point.row as u64,
                    });
                }
            }
        }

        // let mut content = match symbol.get_content().await {
        //     Ok(content) => content,
        //     Err(err) => {
        //         split_errors += 1;
        //         info!("cannot retrieve symbol's content {}", err);
        //         continue;
        //     }
        // };
        //     let symbol_text_maybe = text.get(symbol.full_range.start_byte .. symbol.full_range.end_byte);
        //     if symbol_text_maybe.is_none() {
        //         tracing::warn!("path {:?} range {}..{} is not vaild to get symbol {}", &doc.path, symbol.full_range.start_byte, symbol.full_range.end_byte, symbol.name);
        //         continue;
        //     }
        //     let symbol_text = symbol_text_maybe.unwrap();
        //     if symbol.symbol_type == SymbolType::StructDeclaration {
        //         if let Some(gcx) = global_context.upgrade() {
        //             let full_range = symbol.full_range;
        //             let messages = vec![ChatMessage {
        //                 role: "user".to_string(),
        //                 content: serde_json::to_string(&vec![ContextFile {
        //                     file_name: symbol.file_path.to_str().unwrap().parse().unwrap(),
        //                     file_content: symbol_text.to_string(),
        //                     line1: full_range.start_point.row + 1,
        //                     line2: full_range.end_point.row + 1,
        //                     symbol: symbol.guid.clone(),
        //                     gradient_type: -1,
        //                     usefulness: 100.0,
        //                 }]).unwrap(),
        //             }];
        //             // info!("messages: {:?}", messages);
        //             info!("tokens_limit: {:?}", tokens_limit);
        //             let single_file_mode = true;
        //             let res = postprocess_at_results2(gcx.clone(), messages, tokenizer.clone(), tokens_limit, single_file_mode).await;
        //             if let Some(first) = res.first() {
        //                 info!("{} content was:\n{}", symbol.name, content);
        //                 content = first.file_content.clone();
        //                 info!("content updated:\n{}", content);
        //             }
        //         }
        //     }
        //     if content.len() > self.soft_window {
        //         let mut temp_doc = Document::new(&doc.path, Some("unknown".to_string()));
        //         temp_doc.update_text(&content);
        //         match self.fallback_file_splitter.split(&temp_doc).await {
        //             Ok(mut res) => {
        //                 for r in res.iter_mut() {
        //                     r.start_line += symbol.full_range.start_point.row as u64;
        //                     r.end_line += symbol.full_range.start_point.row as u64;
        //                 }
        //                 chunks.extend(res)
        //             }
        //             Err(err) => {
        //                 info!("{}", err);
        //             }
        //         }
        //         split_using_fallback += 1;
        //         continue;
        //     } else {
        //         split_normally += 1;
        //         chunks.push(SplitResult {
        //             file_path: doc.path.clone(),
        //             window_text: content.clone(),
        //             window_text_hash: str_hash(&content),
        //             start_line: symbol.full_range.start_point.row as u64,
        //             end_line: symbol.full_range.end_point.row as u64,
        //         });
        //     }
        // }
        // let last_30_chars = crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30);
        // let message = format!("split {last_30_chars} by definitions {split_normally}, fallback {split_using_fallback}, errors {split_errors}");
        // info!(message);

        Ok(chunks)
    }
}
