use std::path::PathBuf;
use std::sync::Arc;

use log::info;
use tokio::sync::Mutex as AMutex;
use tree_sitter::Point;

use crate::ast::ast_index::AstIndex;
use crate::ast::structs::{CursorUsagesResult, UsageSearchResultStruct};
use crate::ast::treesitter::parsers::get_parser_by_filename;
use crate::ast::treesitter::structs::SymbolDeclarationStruct;

pub struct AstSearchEngine {
    ast_index: Arc<AMutex<AstIndex>>,
}


impl AstSearchEngine {
    pub fn init(ast_index: Arc<AMutex<AstIndex>>) -> AstSearchEngine {
        AstSearchEngine {
            ast_index
        }
    }

    async fn parse_near_cursor(
        &mut self,
        file_path: &PathBuf,
        code: &str,
        cursor: Point,
    ) -> Result<(Vec<CursorUsagesResult>), String> {
        let mut parser = match get_parser_by_filename(file_path) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let usages = match parser.parse_usages(code).await {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        usages.iter().map(|usage| {
            CursorUsagesResult {
                file_path: file_path.clone(),
                query_text: code.to_string(),
                cursor: cursor.clone(),
                search_results: usages.iter().map(|x| {
                    UsageSearchResultStruct {
                        symbol_path: x.dump_path(),
                        dist_to_cursor: x.distance_to_cursor(),
                    }
                }.collect::<Vec<_>>()),
            }
        })
    }

    pub async fn search(
        &mut self,
        file_path: &PathBuf,
        code: &str,
        cursor: Point,
    ) -> Result<Vec<SymbolDeclarationStruct>, String> {
        let usage_symbols = match self.parse_near_cursor(file_path, code, cursor).await {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        let mut declarations: Vec<SymbolDeclarationStruct> = vec![];
        {
            let ast_index = self.ast_index.clone().lock().await;
            for sym in usage_symbols.iter() {
                declarations.extend(
                    match ast_index.search(sym.query_text.as_str(), 1, Some(file_path.clone())).await {
                        Ok(nodes) => nodes,
                        Err(e) => {
                            info!("Error searching for {}: {}", sym.query_text, e);
                            vec![]
                        }
                    }
                )
            }
        }
        Ok(declarations)
    }
}
