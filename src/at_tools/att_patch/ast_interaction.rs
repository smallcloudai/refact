use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use rand::distributions::Alphanumeric;
use rand::Rng;
use ropey::Rope;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;

use crate::ast::ast_index::RequestSymbolType;
use crate::ast::ast_module::AstModule;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;


pub async fn get_signatures_by_imports_traversal(
    paths: &Vec<String>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Option<Vec<PathBuf>> {
    if let Some(ast_module) = gcx.read().await.ast_module.clone() {
        let mut imported_paths = vec![];
        for filename in paths.iter() {
            if let Ok(path) = PathBuf::from_str(filename) {
                if !path.exists() {
                    continue;
                }
                let doc = Document::new(&path);
                match ast_module
                    .read()
                    .await
                    .imported_file_paths_by_file_path(&doc, 1)
                    .await {
                    Ok(res) => {
                        imported_paths.extend(res.iter().map(|x| x.clone()));
                    }
                    Err(err) => {
                        warn!("Cannot import symbols for path {:?}: {err}", path);
                        continue;
                    }
                };
            } else {
                warn!("Cannot parse path: {filename}");
                continue;
            }
        }
        if !paths.is_empty() {
            Some(imported_paths)
        } else {
            None
        }
    } else {
        None
    }
}

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
    let doc = Document { path: new_filename.clone(), text: Some(file_text.clone()) };
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
