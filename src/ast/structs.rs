use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tree_sitter::Point;
use url::Url;

use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::structs::PointDef;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolsSearchResultStruct {
    pub symbol_declaration: SymbolInformation,
    pub content: String,
    pub sim_to_query: f32,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstCursorSearchResult {
    pub query_text: String,
    pub file_path: PathBuf,
    #[serde(with = "PointDef")]
    pub cursor: Point,
    pub cursor_symbols: Vec<SymbolsSearchResultStruct>,
    pub declaration_symbols: Vec<SymbolsSearchResultStruct>,
    pub declaration_usage_symbols: Vec<SymbolsSearchResultStruct>,
    pub matched_by_name_symbols: Vec<SymbolsSearchResultStruct>,
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
pub struct FileASTMarkup {
    pub file_url: Url,
    pub file_content: String,
    pub symbols_sorted_by_path_len: Vec<SymbolInformation>,
}
