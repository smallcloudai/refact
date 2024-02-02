use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tree_sitter::Point;
use crate::lsp::ast_index::AstIndex;
use crate::lsp::structs::SearchResult;


pub struct AstSearchEngine {
    ast_index: Arc<AMutex<AstIndex>>,
}


impl AstSearchEngine {
    pub fn init(ast_index: Arc<AMutex<AstIndex>>) -> AstSearchEngine {
        AstSearchEngine {
            ast_index
        }
    }


    pub fn search(
        &mut self,
        query: &str,
        filename: &PathBuf,
        cursor: Point
    ) -> Vec<SearchResult> {

    }
}
