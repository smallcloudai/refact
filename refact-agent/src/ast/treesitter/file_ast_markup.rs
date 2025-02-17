// use std::path::PathBuf;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;

pub struct FileASTMarkup {
    // pub file_path: PathBuf,
    // pub file_content: String,
    pub symbols_sorted_by_path_len: Vec<SymbolInformation>,
}
