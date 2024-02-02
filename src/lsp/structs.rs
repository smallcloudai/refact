use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tree_sitter::Point;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolDeclarationStruct {
    pub id: usize,
    pub node_type: String,
    pub name: String,
    pub content: String,
    pub start_point: Point,
    pub end_point: Point,
    pub path: String,
    pub parent_ids: Option<Vec<usize>>,
    pub namespaces_name: Option<Vec<String>>,
    pub meta_path: String
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query_text: String,
    pub filename: PathBuf,
    pub cursor: Point,
    pub search_results: Vec<SymbolDeclarationStruct>,
}
