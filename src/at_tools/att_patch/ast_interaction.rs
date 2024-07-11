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

async fn symbols_to_signatures_context(symbols: &Vec<SymbolInformation>) -> String {
    let mut context: String = "".to_string();
    for s in symbols.iter() {
        let decl_sign = match s.get_declaration_content_from_file().await {
            Ok(sign) => sign,
            Err(err) => {
                warn!("Cannot get a content for symbol {:?}: {err}", s.name);
                continue;
            }
        };
        context.push_str(&format!("```\n{decl_sign}\n```\n"))
    }
    context
}

pub async fn get_signatures_by_symbol_names(
    symbol_names: &Vec<String>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Option<String> {
    return if let Some(ast_module) = gcx.read().await.ast_module.clone() {
        let mut symbols = vec![];
        for name in symbol_names.iter() {
            let res = match ast_module
                .read()
                .await
                .search_by_name(name.clone(), RequestSymbolType::Declaration, false, 1)
                .await {
                Ok(s) => s.search_results
                    .get(0)
                    .map(|x| x.symbol_declaration.clone()),
                Err(_) => None
            };
            if let Some(s) = res {
                symbols.push(s.clone());
            }
        }
        if !symbols.is_empty() {
            Some(symbols_to_signatures_context(&symbols).await)
        } else {
            None
        }
    } else {
        None
    };
}

pub async fn get_signatures_by_imports_traversal(
    paths: &Vec<String>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Option<String> {
    return if let Some(ast_module) = gcx.read().await.ast_module.clone() {
        let mut symbols = vec![];
        for filename in paths.iter() {
            if let Ok(path) = PathBuf::from_str(filename) {
                let doc = Document::new(&path);
                match ast_module
                    .read()
                    .await
                    .decl_symbols_from_imports_by_file_path(&doc, 1)
                    .await {
                    Ok(s) => {
                        s.search_results
                            .iter()
                            .map(|x| {
                                symbols.push(x.symbol_declaration.clone());
                                s.clone()
                            })
                            .collect::<Vec<_>>()
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
        if !symbols.is_empty() {
            Some(symbols_to_signatures_context(&symbols).await)
        } else {
            None
        }
    } else {
        None
    };
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
