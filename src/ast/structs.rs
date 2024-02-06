use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tree_sitter::Point;

use crate::ast::treesitter::structs::{PointDef, SymbolDeclarationStruct};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageSearchResultStruct {
    pub symbol_path: String,
    pub dist_to_cursor: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolsSearchResultStruct {
    pub symbol_path: String,
    pub content: String,
    pub lev_dist_to_query: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CursorUsagesResult {
    pub query_text: String,
    pub file_path: PathBuf,
    #[serde(with = "PointDef")]
    pub cursor: Point,
    pub search_results: Vec<UsageSearchResultStruct>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query_text: String,
    pub file_path: PathBuf,
    #[serde(with = "PointDef")]
    pub cursor: Point,
    pub search_results: Vec<SymbolDeclarationStruct>,
}
