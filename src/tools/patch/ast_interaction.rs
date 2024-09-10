use std::path::PathBuf;
use std::sync::Arc;

use rand::distributions::Alphanumeric;
use rand::Rng;
use ropey::Rope;
use tokio::sync::RwLock as ARwLock;

use crate::ast::ast_module::AstModule;
use crate::ast::linters::lint;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::files_in_workspace::Document;

pub async fn parse_and_get_error_symbols(
    ast_module: Arc<ARwLock<AstModule>>,
    path: &PathBuf,
    file_text: &Rope,
) -> Result<Vec<SymbolInformation>, String> {
    let dummy_filename = PathBuf::from(rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>());
    let new_filename = dummy_filename.with_extension(
        path.extension().unwrap_or_default()
    );
    let doc = Document { doc_path: new_filename.clone(), doc_text: Some(file_text.clone()) };
    match ast_module.read()
        .await
        .file_markup(&doc)
        .await {
        Ok(symbols) => Ok(symbols
            .symbols_sorted_by_path_len
            .into_iter()
            .filter(|x| x.is_error)
            .collect::<Vec<_>>()),
        Err(err) => Err(err)
    }
}


pub fn lint_and_get_error_messages(
    path: &PathBuf,
    file_text: &Rope,
) -> Vec<String> {
    let dummy_filename = PathBuf::from(rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>());
    let new_filename = dummy_filename.with_extension(
        path.extension().unwrap_or_default()
    );
    let doc = Document { doc_path: new_filename.clone(), doc_text: Some(file_text.clone()) };
    match lint(&doc) {
        Ok(_) => vec![],
        Err(problems) => problems,
    }
}
