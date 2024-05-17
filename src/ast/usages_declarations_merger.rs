use std::cmp::Ordering;
use std::path::PathBuf;
use hashbrown::HashMap;

use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceRc, FunctionDeclaration};
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
    symbol: &AstSymbolInstanceRc,
    caller_guid: &Uuid,
    guid_by_symbols: &HashMap<Uuid, AstSymbolInstanceRc>,
    extra_search_index: &HashMap<(String, Uuid, SymbolType), Uuid>,
) -> Option<Uuid> {
    let (symbol_type, name, is_error_node) = {
        let s = symbol.borrow();
        (s.symbol_type().to_owned(), s.name().to_owned(), s.is_error())
    };
    let search_symbol_types: Vec<SymbolType> = if !is_error_node {
        match symbol_type {
            SymbolType::FunctionCall => { vec![SymbolType::FunctionDeclaration] }
            SymbolType::VariableUsage => { vec![SymbolType::VariableDefinition, SymbolType::ClassFieldDeclaration] }
            _ => { return None; }
        }
    } else {
        vec![SymbolType::ClassFieldDeclaration, SymbolType::VariableDefinition, SymbolType::FunctionDeclaration]
    };

    let caller_symbol = match guid_by_symbols.get(caller_guid) {
        Some(s) => { s }
        None => { return None; }
    };

    let (symbol_type, linked_decl_guid) = {
        let s_ref = caller_symbol.borrow();
        (s_ref.symbol_type().clone(), s_ref.get_linked_decl_guid().clone())
    };
    let decl_symbol = match symbol_type {
        SymbolType::FunctionCall => {
            linked_decl_guid
                .map(|guid| {
                    let symbol_ref = guid_by_symbols.get(&guid)?.borrow();
                    symbol_ref
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
            linked_decl_guid
                .as_ref()
                .map(|guid| guid_by_symbols.get(guid))?
        }
        _ => None
    };

    let decl_symbol_parent = decl_symbol?
        .borrow()
        .parent_guid()
        .as_ref()
        .map(|guid| { guid_by_symbols.get(guid) })??;
    let decl_symbol_parent_guid = decl_symbol_parent.borrow().guid().clone();

    search_symbol_types
        .iter()
        .map(|symbol_type| {
            let search_q = (name.clone(), decl_symbol_parent_guid, symbol_type.clone());
            extra_search_index.get(&search_q)
        })
        .filter_map(|guid| guid)
        .cloned()
        .collect::<Vec<_>>()
        .first()
        .cloned()
}

fn find_decl_by_name_for_single_path(
    name: &str,
    parent_guid: &Uuid,
    search_symbol_type: &SymbolType,
    is_error_node: bool,
    file_path: &PathBuf,
    guid_by_symbols: &HashMap<Uuid, AstSymbolInstanceRc>,
    extra_search_index: &HashMap<(String, Uuid, String), AstSymbolInstanceRc>,
) -> Option<Uuid> {
    let mut current_parent_guid = parent_guid.clone();
    loop {
        let search_q = (
            name.to_string(),
            current_parent_guid.clone(),
            file_path.to_str().unwrap_or_default().to_string()
        );
        if let Some(s) = extra_search_index
            .get(&search_q)
            .map(|s| s.clone()) {
            let s_ref = s.borrow();
            let valid_type = is_error_node || (s_ref.symbol_type() == *search_symbol_type);
            if valid_type {
                return Some(s_ref.guid().clone())
            }
        }
        if current_parent_guid.is_nil() {
            break;
        } else {
            current_parent_guid = match guid_by_symbols.get(&current_parent_guid) {
                Some(s) => {
                    s.borrow().parent_guid().clone().unwrap_or(Uuid::default())
                }
                None => { Uuid::default() }
            };
            continue;
        }
    }
    None
}

pub fn find_decl_by_name(
    symbol: &AstSymbolInstanceRc,
    path_by_symbols: &HashMap<PathBuf, Vec<AstSymbolInstanceRc>>,
    guid_by_symbols: &HashMap<Uuid, AstSymbolInstanceRc>,
    extra_search_index: &HashMap<(String, Uuid, String), AstSymbolInstanceRc>,
    top_n_files: usize,
) -> Option<Uuid> {
    let (file_path, parent_guid, name, is_function, is_error_node) =  {
        let s_ref = symbol.borrow();
        (s_ref.file_path().clone(),
         s_ref.parent_guid().clone().unwrap_or_default(),
         s_ref.name().to_string(),
         s_ref.symbol_type() == SymbolType::FunctionCall,
         s_ref.is_error())
    };
    let search_symbol_type = match is_function {
        true => SymbolType::FunctionDeclaration,
        false => SymbolType::VariableDefinition,
    };
    let file_iterator = if top_n_files > 1 {
        FilePathIterator::new(file_path.clone(), path_by_symbols.keys().map(|x|x.clone()).collect::<Vec<PathBuf>>()).collect::<Vec<_>>()
    } else {
        vec![file_path.clone()]
    };
    for file in file_iterator.iter().take(top_n_files) {
        let current_parent_guid = match file_path == *file {
            true => parent_guid.clone(),
            false => Uuid::default()
        };
        match find_decl_by_name_for_single_path(
            &name,
            &current_parent_guid,
            &search_symbol_type,
            is_error_node,
            file,
            guid_by_symbols,
            extra_search_index,
        ) {
            Some(guid) => { return Some(guid); }
            None => { continue; }
        }
    }
    None
}
