use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures_util::StreamExt;
use itertools::Itertools;

use url::Url;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, AstSymbolInstanceArc, FunctionCall, FunctionDeclaration, TypeDef, VariableUsage};
use crate::ast::treesitter::structs::SymbolType;

struct FilePathIterator {
    paths: Vec<PathBuf>,
    index: usize, // Current position in the list
}

impl FilePathIterator {
    fn new(start_path: PathBuf, mut all_paths: Vec<PathBuf>) -> FilePathIterator {
        all_paths.sort_by(|a, b| {
            FilePathIterator::compare_paths(&start_path, a, b)
        });

        FilePathIterator {
            paths: all_paths,
            index: 0,
        }
    }

    fn compare_paths(start_path: &PathBuf, a: &PathBuf, b: &PathBuf) -> Ordering {
        let start_components: Vec<_> = start_path.components().collect();
        let a_components: Vec<_> = a.components().collect();
        let b_components: Vec<_> = b.components().collect();

        let a_distance = a_components
            .iter()
            .zip(&start_components)
            .take_while(|(a, b)| a == b)
            .count();
        let b_distance = b_components.iter()
            .zip(&start_components)
            .take_while(|(a, b)| a == b)
            .count();

        a_distance.cmp(&b_distance).reverse()
    }
}

impl Iterator for FilePathIterator {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.paths.len() {
            let path = self.paths[self.index].clone();
            self.index += 1;
            Some(path)
        } else {
            None
        }
    }
}

pub fn find_decl_by_caller_guid(
    caller_guid: &str,
    name: &str,
    symbol_type: &SymbolType,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
) -> Option<String> {
    let search_symbol_type = match symbol_type {
        SymbolType::FunctionCall => { SymbolType::FunctionDeclaration }
        SymbolType::VariableUsage => { SymbolType::ClassFieldDeclaration }
        _ => { return None; }
    };
    let caller_symbol = match guid_by_symbols.get(caller_guid) {
        Some(s) => { s }
        None => { return None; }
    };
    let decl_symbol = match caller_symbol.blocking_read().symbol_type() {
        SymbolType::FunctionCall => {
            caller_symbol
                .blocking_read()
                .get_linked_decl_guid()
                .as_ref()
                .map(|guid| {
                    guid_by_symbols
                        .get(guid)?
                        .blocking_read()
                        .as_any()
                        .downcast_ref::<FunctionDeclaration>()?
                        .return_type
                        .as_ref()
                        .map(|obj| obj.guid
                            .as_ref()
                            .map(|g| guid_by_symbols.get(g)))??
                })?
        }
        SymbolType::VariableUsage => {
            caller_symbol
                .blocking_read()
                .get_linked_decl_guid()
                .as_ref()
                .map(|guid| guid_by_symbols.get(guid))?
        }
        _ => None
    };

    let decl_symbol_parent = decl_symbol?
        .blocking_read()
        .parent_guid()
        .as_ref()
        .map(|guid| { guid_by_symbols.get(guid) })??;
    return match guid_by_symbols
        .iter()
        .filter(|(_, symbol)| {
            let s_ref = symbol.blocking_read();
            s_ref.symbol_type() == search_symbol_type
                && s_ref.parent_guid().clone().unwrap_or_default() == decl_symbol_parent.blocking_read().guid()
                && s_ref.name() == name
        })
        .map(|(_, symbol)| symbol)
        .next() {
        Some(s) => { Some(s.blocking_read().guid().to_string()) }
        None => { return None; }
    };
}

fn find_decl_by_name_for_single_path(
    name: &str,
    parent_guid: &str,
    search_symbol_type: &SymbolType,
    symbols: &Vec<AstSymbolInstanceArc>,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
) -> Option<String> {
    let mut current_parent_guid = parent_guid.to_string();
    loop {
        let found_symbol = match symbols
            .iter()
            .filter(|s| {
                let s_ref = s.blocking_read();
                s_ref.symbol_type() == *search_symbol_type
                    && s_ref.parent_guid().clone().unwrap_or("".to_string()) == current_parent_guid
                    && s_ref.name() == name
            })
            .next() {
            Some(s) => {
                s
            }
            None => {
                if current_parent_guid.is_empty() {
                    break;
                } else {
                    current_parent_guid = match guid_by_symbols.get(&current_parent_guid) {
                        Some(s) => {
                            s.blocking_read().parent_guid().clone().unwrap_or("".to_string())
                        }
                        None => { "".to_string() }
                    };
                    continue;
                }
            }
        };
        return Some(found_symbol.blocking_read().guid().to_string());
    }
    None
}

pub fn find_decl_by_name(
    name: &str,
    file_path: &Url,
    parent_guid: &str,
    is_function: bool,
    path_by_symbols: &HashMap<Url, Vec<AstSymbolInstanceArc>>,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
) -> Option<String> {
    let search_symbol_type = match is_function {
        true => SymbolType::FunctionDeclaration,
        false => SymbolType::VariableDefinition,
    };
    let file_iterator = FilePathIterator::new(
        file_path.to_file_path().unwrap_or_default(),
        path_by_symbols
            .iter()
            .filter_map(|(url, _)| url.to_file_path().ok())
            .collect::<Vec<_>>(),
    );
    for file in file_iterator {
        let url = match Url::from_file_path(file) {
            Ok(url) => url,
            Err(_) => { continue; }
        };
        let current_parent_guid = match *file_path == url {
            true => parent_guid,
            false => ""
        };
        let symbols = match path_by_symbols.get(&url) {
            Some(symbols) => symbols,
            None => { continue; }
        };
        match find_decl_by_name_for_single_path(
            name,
            current_parent_guid,
            &search_symbol_type,
            symbols,
            guid_by_symbols
        ) {
            Some(guid) => { return Some(guid); }
            None => { continue; }
        }
    }
    None
}