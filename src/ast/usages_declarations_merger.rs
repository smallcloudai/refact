use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use url::Url;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, FunctionDeclaration};
use crate::ast::treesitter::structs::SymbolType;

pub struct FilePathIterator {
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

    pub fn compare_paths(start_path: &PathBuf, a: &PathBuf, b: &PathBuf) -> Ordering {
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
    symbol: &AstSymbolInstanceArc,
    caller_guid: &str,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
) -> Option<String> {
    let (symbol_type, name, is_error_node) = {
        let s = symbol.read().expect("the data might be broken");
        (s.symbol_type().to_owned(), s.name().to_owned(), s.is_error())
    };
    let search_symbol_type = match symbol_type {
        SymbolType::FunctionCall => { SymbolType::FunctionDeclaration }
        SymbolType::VariableUsage => { SymbolType::ClassFieldDeclaration }
        _ => { return None; }
    };
    let caller_symbol = match guid_by_symbols.get(caller_guid) {
        Some(s) => { s }
        None => { return None; }
    };

    let decl_symbol = match caller_symbol
        .read().expect("the data might be broken")
        .symbol_type() {
        SymbolType::FunctionCall => {
            let linked_decl_guid = caller_symbol
                .read().expect("the data might be broken")
                .get_linked_decl_guid()
                .to_owned();
            linked_decl_guid
                .map(|guid| {
                    guid_by_symbols
                        .get(&guid)?
                        .read().expect("the data might be broken")
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
                .read().expect("the data might be broken")
                .get_linked_decl_guid()
                .as_ref()
                .map(|guid| guid_by_symbols.get(guid))?
        }
        _ => None
    };

    let decl_symbol_parent = decl_symbol?
        .read().expect("the data might be broken")
        .parent_guid()
        .as_ref()
        .map(|guid| { guid_by_symbols.get(guid) })??;
    return match guid_by_symbols
        .iter()
        .filter(|(_, symbol)| {
            let s_ref = symbol.read().expect("the data might be broken");
            let valid_type = is_error_node || (s_ref.symbol_type() == search_symbol_type);
            valid_type
                && s_ref.parent_guid().clone().unwrap_or_default() == decl_symbol_parent.read().expect("the data might be broken").guid()
                && s_ref.name() == name
        })
        .map(|(_, symbol)| symbol)
        .next() {
        Some(s) => { Some(s.read().expect("the data might be broken").guid().to_string()) }
        None => { return None; }
    };
}

fn find_decl_by_name_for_single_path(
    name: &str,
    parent_guid: &str,
    search_symbol_type: &SymbolType,
    is_error_node: bool,
    file_url: &Url,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
    extra_search_index: &HashMap<(String, String, String), AstSymbolInstanceArc>,
) -> Option<String> {
    let mut current_parent_guid = parent_guid.to_string();
    loop {
        let search_q = (
            name.to_string(),
            current_parent_guid.clone(),
            file_url.to_file_path().unwrap_or_default().to_str().unwrap_or_default().to_string()
        );
        match extra_search_index
            .get(&search_q)
            .map(|s| s.clone()) {
            Some(s) => {
                let s_ref = s.read().expect("the data might be broken");
                let valid_type = is_error_node || (s_ref.symbol_type() == *search_symbol_type);
                return if valid_type {
                    Some(s_ref.guid().to_string())
                } else {
                    None
                }
            }
            None => {
                if current_parent_guid.is_empty() {
                    break;
                } else {
                    current_parent_guid = match guid_by_symbols.get(&current_parent_guid) {
                        Some(s) => {
                            s.read().expect("the data might be broken").parent_guid().clone().unwrap_or("".to_string())
                        }
                        None => { "".to_string() }
                    };
                    continue;
                }
            }
        };
    }
    None
}

pub fn find_decl_by_name(
    symbol: &AstSymbolInstanceArc,
    path_by_symbols: &HashMap<Url, Vec<AstSymbolInstanceArc>>,
    guid_by_symbols: &HashMap<String, AstSymbolInstanceArc>,
    extra_search_index: &HashMap<(String, String, String), AstSymbolInstanceArc>,
    top_n_files: usize,
) -> Option<String> {
    let (file_path, parent_guid, name, is_function, is_error_node) = match symbol.read() {
        Ok(s) => {
            (s.file_url().to_owned(),
             s.parent_guid().to_owned().unwrap_or_default(),
             s.name().to_owned(),
             s.symbol_type() == SymbolType::FunctionCall,
             s.is_error())
        }
        Err(_) => { return None; }
    };
    let search_symbol_type = match is_function {
        true => SymbolType::FunctionDeclaration,
        false => SymbolType::VariableDefinition,
    };
    let file_iterator = if top_n_files > 1 {
        FilePathIterator::new(
            file_path.to_file_path().unwrap_or_default(),
            path_by_symbols
                .iter()
                .filter_map(|(url, _)| url.to_file_path().ok())
                .collect::<Vec<_>>(),
        ).collect::<Vec<_>>()
    } else {
        vec![file_path.to_file_path().unwrap_or_default()]
    };
    for file in file_iterator.iter().take(top_n_files) {
        let url = match Url::from_file_path(file) {
            Ok(url) => url,
            Err(_) => { continue; }
        };
        let current_parent_guid = match file_path == url {
            true => parent_guid.clone(),
            false => "".to_string()
        };
        match find_decl_by_name_for_single_path(
            &name,
            &current_parent_guid,
            &search_symbol_type,
            is_error_node,
            &url,
            guid_by_symbols,
            extra_search_index,
        ) {
            Some(guid) => { return Some(guid); }
            None => { continue; }
        }
    }
    None
}