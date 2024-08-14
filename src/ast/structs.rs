use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tree_sitter::Point;

use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::structs::PointDef;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolsSearchResultStruct {
    pub symbol_declaration: SymbolInformation,
    pub content: String,
    pub usefulness: f32,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstCursorSearchResult {
    pub query_text: String,
    pub cursor_symbols: Vec<SymbolsSearchResultStruct>,
    pub file_path: PathBuf,
    #[serde(with = "PointDef")]
    pub cursor: Point,
    pub bucket_declarations: Vec<SymbolsSearchResultStruct>,        // types and functions around cursor, found in indexes (matched by guid and by name)
    pub bucket_usage_of_same_stuff: Vec<SymbolsSearchResultStruct>, // sum of (4) find anything that uses the same types and functions and (5) find function calls by name
    pub bucket_high_overlap: Vec<SymbolsSearchResultStruct>,        // (6) declarations with high symbols overlap
    pub bucket_imports: Vec<SymbolsSearchResultStruct>,             // symbols from imports
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstDeclarationSearchResult {
    pub query_text: String,
    pub exact_matches: Vec<SymbolsSearchResultStruct>,
    pub fuzzy_matches: Vec<SymbolsSearchResultStruct>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstReferencesSearchResult {
    pub query_text: String,
    pub declaration_exact_matches: Vec<SymbolsSearchResultStruct>,
    pub declaration_fuzzy_matches: Vec<SymbolsSearchResultStruct>,
    pub references_for_exact_matches: Vec<SymbolsSearchResultStruct>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AstQuerySearchResult {
    pub query_text: String,
    pub search_results: Vec<SymbolsSearchResultStruct>,
    pub refs_n: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileReferencesResult {
    pub file_path: PathBuf,
    pub symbols: Vec<SymbolInformation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileASTMarkup {
    pub file_path: PathBuf,
    pub file_content: String,
    pub symbols_sorted_by_path_len: Vec<SymbolInformation>,
}
