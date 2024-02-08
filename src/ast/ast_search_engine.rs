use std::path::PathBuf;
use std::sync::Arc;

use log::info;
use tokio::sync::Mutex as AMutex;
use tree_sitter::Point;

use crate::ast::ast_index::AstIndex;
use crate::ast::structs::{CursorUsagesResult, SymbolsSearchResultStruct, UsageSearchResultStruct};
use crate::ast::treesitter::parsers::get_parser_by_filename;

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
    ) -> Result<CursorUsagesResult, String> {
        let mut parser = match get_parser_by_filename(file_path) {
            Ok(parser) => parser,
            Err(err) => {
                return Err(err.message);
            }
        };
        let usages = match parser.parse_usages(code) {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        Ok(CursorUsagesResult {
            file_path: file_path.clone(),
            query_text: code.to_string(),
            cursor: cursor.clone(),
            search_results: usages
                .iter()
                .map(|x| {
                    UsageSearchResultStruct {
                        symbol_path: x.dump_path(),
                        dist_to_cursor: x.distance_to_cursor(&cursor),
                        type_str: x.type_str()
                    }
                })
                .collect::<Vec<UsageSearchResultStruct>>(),
        })
    }

    pub async fn search(
        &mut self,
        file_path: &PathBuf,
        code: &str,
        cursor: Point,
    ) -> Result<(Vec<SymbolsSearchResultStruct>, Vec<UsageSearchResultStruct>), String> {
        let usage_result = match self.parse_near_cursor(file_path, code, cursor).await {
            Ok(usages) => usages,
            Err(e) => {
                return Err(format!("Error parsing {}: {}", file_path.display(), e));
            }
        };
        let mut declarations: Vec<SymbolsSearchResultStruct> = vec![];
        {
            let ast_index = self.ast_index.clone();
            let ast_index_locked  = ast_index.lock().await;
            for sym in usage_result.search_results.iter() {
                declarations.extend(
                    match ast_index_locked.search(sym.symbol_path.as_str(), 1, Some(file_path.clone())).await {
                        Ok(nodes) => nodes,
                        Err(e) => {
                            info!("Error searching for {}: {}", sym.symbol_path.as_str(), e);
                            vec![]
                        }
                    }
                )
            }
        }
        Ok((declarations, usage_result.search_results))
    }
}
