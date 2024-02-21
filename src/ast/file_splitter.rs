use md5;
use ropey::Rope;
use tracing::info;

use crate::ast::treesitter::parsers::get_parser_by_filename;
use crate::files_in_workspace::{Document, DocumentInfo};
use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::structs::SplitResult;

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub struct AstBasedFileSplitter {
    soft_window: usize,
    hard_window: usize,
    fallback_file_splitter: FileSplitter
}

impl AstBasedFileSplitter {
    pub fn new(window_size: usize, soft_limit: usize) -> Self {
        Self {
            soft_window: window_size,
            hard_window: window_size + soft_limit,
            fallback_file_splitter: FileSplitter::new(window_size, soft_limit)
        }
    }

    pub async fn split(&self, doc_info: &DocumentInfo) -> Result<Vec<SplitResult>, String> {
        let path = doc_info.get_path();
        let mut parser = match get_parser_by_filename(&doc_info.get_path()) {
            Ok(parser) => parser,
            Err(_) => {
                info!("cannot find a parser for {:?}, using simple file splitter", doc_info.get_path());
                return self.fallback_file_splitter.split(doc_info).await;
            }
        };
        let text = match doc_info.read_file().await {
            Ok(s) => s,
            Err(err) => {
                return Err(err.to_string())
            }
        };
        let declarations = match parser.parse_declarations(text.as_str(), &path) {
            Ok(declarations) => declarations,
            Err(_) => {
                info!("cannot parse {:?}, using simple file splitter", doc_info.get_path());
                return self.fallback_file_splitter.split(doc_info).await;
            }
        };

        let mut chunks = Vec::new();
        for (path, declaration) in declarations.iter() {
            let content = match declaration.get_content().await {
                Ok(content) => content,
                Err(err) => {
                    info!("cannot retrieve symbol's content {}", err);
                    continue;
                }
            };
            if content.len() > self.soft_window {
                info!("too large content size to vectorize the chunk, using simple file splitter {}", path);
                let temp_doc_info = DocumentInfo {
                    uri: doc_info.uri.clone(),
                    document: Some(Document {
                        language_id: "unknown".to_string(),
                        text: Rope::from_str(&content)
                    })
                };
                match self.fallback_file_splitter.split(&temp_doc_info).await {
                    Ok(res) => chunks.extend(res),
                    Err(err) => {
                        info!("{}", err);
                    }
                }
                continue;
            }

            chunks.push(SplitResult {
                file_path: doc_info.get_path(),
                window_text: content.clone(),
                window_text_hash: str_hash(&content),
                start_line: declaration.definition_info.range.start_point.row as u64,
                end_line: declaration.definition_info.range.end_point.row as u64,
            });
        }

        Ok(chunks)
    }
}
