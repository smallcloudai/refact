use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tree_sitter::Point;
use url::Url;

use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::structs::PointDef;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageSearchResultStruct {
    pub symbol_path: String,
    pub dist_to_cursor: usize,
    pub type_str: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolsSearchResultStruct {
    pub symbol_declaration: SymbolInformation,
    pub content: String,
    pub sim_to_query: f32,
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
pub struct AstCursorSearchResult {
    pub query_text: String,
    pub file_path: PathBuf,
    #[serde(with = "PointDef")]
    pub cursor: Point,
    pub cursor_symbols: Vec<UsageSearchResultStruct>,
    pub search_results: Vec<SymbolsSearchResultStruct>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstQuerySearchResult {
    pub query_text: String,
    pub search_results: Vec<SymbolsSearchResultStruct>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileReferencesResult {
    pub file_path: PathBuf,
    pub symbols: Vec<SymbolInformation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RowMarkup {
    pub symbols_guid: Vec<String>,  // is sorted parent to child
    pub line_content: String,
    pub is_signature: bool
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileASTMarkup {
    pub file_url: Url,
    pub symbols: Vec<SymbolInformation>,
    pub rows_markup: HashMap<usize, RowMarkup>,
}


