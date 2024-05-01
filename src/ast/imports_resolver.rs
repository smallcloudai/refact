use std::collections::HashMap;
use std::path::{Path, PathBuf};

use itertools::Itertools;
use tracing::info;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, AstSymbolInstanceArc, ImportDeclaration, read_symbol};
use crate::ast::treesitter::language_id::LanguageId;

pub fn combine_paths(file_path: &PathBuf, import_path: &PathBuf) -> PathBuf {
    fn find_unique_prefix(path_1: &PathBuf, path_2: &PathBuf) -> PathBuf {
        let components1 = path_1.components().collect::<Vec<_>>();
        let components2 = path_2.components().collect::<Vec<_>>();

        let mut prefix = PathBuf::new();
        for (i, component) in components1.iter().enumerate() {
            if i >= components2.len() || component != &components2[i] {
                prefix.push(component.as_os_str());
            } else {
                break;
            }
        }
        prefix
    }

    fn find_common_part(path_1: &PathBuf, path_2: &PathBuf) -> PathBuf {
        let components1 = path_1.components().collect::<Vec<_>>();
        let components2 = path_2.components().collect::<Vec<_>>();

        let mut common = PathBuf::new();
        let min_length = components1.len().min(components2.len());

        for i in 0..min_length {
            if components1[i] == components2[i] {
                common.push(components1[i].as_os_str());
            } else {
                break;
            }
        }
        common
    }

    fn find_unique_suffix(path_1: &PathBuf, path_2: &PathBuf) -> PathBuf {
        let common_length = find_common_part(path_1, path_2).components().count();
        let components2 = path_2.components().collect::<Vec<_>>();

        let mut suffix = PathBuf::new();
        for component in components2.iter().skip(common_length) {
            suffix.push(component.as_os_str());
        }
        suffix
    }

    let parent_file_path = match file_path.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            return import_path.clone();
        }
    };

    let unique_prefix = find_unique_prefix(&parent_file_path, import_path);
    let common_part = find_common_part(&parent_file_path, import_path);
    let unique_suffix = find_unique_suffix(&parent_file_path, import_path);

    let mut result_path = PathBuf::new();
    result_path.extend(unique_prefix.components());
    result_path.extend(common_part.components());
    result_path.extend(unique_suffix.components());

    result_path
}

pub fn relative_combine_paths(file_path: &PathBuf, import_path: &PathBuf) -> PathBuf {
    let mut parent_file_path = match file_path.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            return import_path.clone();
        }
    };
    for component in import_path.components() {
        parent_file_path.push(component);
    }
    parent_file_path
}

pub fn build_prefixes(path: &Path) -> Vec<PathBuf> {
    let mut current_path = PathBuf::new();
    let mut prefixes = Vec::new();

    for component in path.components() {
        current_path.push(component.as_os_str());
        prefixes.push(current_path.clone());
    }

    prefixes
}

pub fn top_n_prefixes(paths: &Vec<PathBuf>, n: usize) -> Vec<(PathBuf, usize)> {
    let mut prefix_count = HashMap::new();

    for path in paths.iter() {
        for prefix in build_prefixes(&path) {
            *prefix_count.entry(prefix).or_insert(0) += 1;
        }
    }

    let mut counted_prefixes: Vec<(PathBuf, usize)> = prefix_count.into_iter().collect();
    counted_prefixes.sort_by(|a, b| b.1.cmp(&a.1));
    let max_elements = counted_prefixes
        .get(0)
        .map(|x| x.1)
        .unwrap_or(0);
    let max_size_with_max_elements_path = counted_prefixes
        .clone()
        .iter()
        .filter(|x| x.1 == max_elements)
        .max_by(|a, b| a.0.to_str().unwrap_or_default().cmp(b.0.to_str().unwrap_or_default()))
        .map(|x| x.0.clone())
        .unwrap_or_default();
    counted_prefixes
        .into_iter()
        .filter(|x| x.0 == max_size_with_max_elements_path || x.1 < max_elements)
        .take(n)
        .collect()
}

pub fn try_find_file_path(
    possible_file_paths: &Vec<PathBuf>,
    path_by_symbols: &HashMap<PathBuf, Vec<AstSymbolInstanceArc>>,
    paths_str: &Vec<String>,
) -> Option<PathBuf> {
    for path in possible_file_paths {
        if path_by_symbols.contains_key(path) {
            return Some(path.clone());
        }
    }

    // no exact match, try to found by path inclusion (could be slow, O(n))
    let possible_file_paths_str = possible_file_paths
        .iter()
        .filter_map(|path| path.to_str())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    for file_path in paths_str.iter() {
        for import_path in possible_file_paths_str.iter() {
            if file_path.contains(import_path) {
                return Some(PathBuf::from(file_path.clone()));
            }
        }
    }
    None
}

pub fn possible_filepath_candidates(
    path_by_symbols: &HashMap<PathBuf, Vec<AstSymbolInstanceArc>>,
    prefixes: &Vec<(PathBuf, usize)>,
    min_prefix: &Option<PathBuf>,
    file_path: &PathBuf,
    path_components: &Vec<String>,
    language: &LanguageId,
) -> Vec<PathBuf> {
    match language {
        LanguageId::Cpp => {
            let import_path = path_components.iter().collect::<PathBuf>();
            let mut path = combine_paths(&file_path, &import_path);
            if !path_by_symbols.contains_key(&path) {
                path = relative_combine_paths(&file_path, &import_path)
            };
            if !path_by_symbols.contains_key(&path) {
                path = import_path.canonicalize().unwrap_or(import_path.clone());
            };
            let mut possible_file_paths = vec![path];
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(prefix_p.join(import_path.clone()));
            }
            possible_file_paths
        }
        LanguageId::JavaScript => {
            let import_path = path_components.iter().collect::<PathBuf>();
            let mut possible_file_paths = vec![];
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(prefix_p.join(import_path.clone()));
                if let Some(parent_p) = import_path.parent() {
                    possible_file_paths.push(prefix_p.join(parent_p));
                }
            }
            let mut start_path = file_path.clone();
            loop {
                if let Some(parent_p) = start_path.parent() {
                    start_path = parent_p.to_path_buf();
                    if min_prefix.clone().map(|x| x == start_path).unwrap_or(false) {
                        break;
                    }
                } else {
                    break;
                }
                possible_file_paths.push(start_path.join(import_path.clone()));
                if let Some(parent_p) = import_path.parent() {
                    possible_file_paths.push(start_path.join(parent_p));
                }
            }
            possible_file_paths.iter().unique().map(|x| {
                let mut x = x.clone();
                x.set_extension("js");
                x.canonicalize().unwrap_or(x)
            }).collect::<Vec<_>>()
        }
        LanguageId::Python => {
            let import_path = path_components.iter().collect::<PathBuf>();
            let mut possible_file_paths = vec![];
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(prefix_p.join(import_path.clone()));
                if path_components.len() > 1 {
                    if let Some(parent_p) = import_path.parent() {
                        possible_file_paths.push(prefix_p.join(parent_p));
                    }
                }
            }
            let mut start_path = file_path.clone();
            loop {
                if let Some(parent_p) = start_path.parent() {
                    start_path = parent_p.to_path_buf();
                    if min_prefix.clone().map(|x| x == start_path).unwrap_or(false) {
                        break;
                    }
                } else {
                    break;
                }
                possible_file_paths.push(start_path.join(import_path.clone()));
                if path_components.len() > 1 {
                    if let Some(parent_p) = import_path.parent() {
                        possible_file_paths.push(start_path.join(parent_p));
                    }
                }
            }
            possible_file_paths.iter().unique().map(|x| {
                let mut x = x.clone();
                x.set_extension("py");
                x.canonicalize().unwrap_or(x)
            }).collect::<Vec<_>>()
        }
        LanguageId::Rust => {
            let import_path = path_components.iter().filter(|c| *c != "crate").collect::<PathBuf>();
            let mut possible_file_paths = vec![];
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(prefix_p.join(import_path.clone()));
                if path_components.len() > 1 {
                    if let Some(parent_p) = import_path.parent() {
                        possible_file_paths.push(prefix_p.join(parent_p));
                    }
                }
            }
            let mut start_path = file_path.clone();
            loop {
                if let Some(parent_p) = start_path.parent() {
                    start_path = parent_p.to_path_buf();
                    if min_prefix.clone().map(|x| x == start_path).unwrap_or(false) {
                        break;
                    }
                } else {
                    break;
                }
                possible_file_paths.push(start_path.join(import_path.clone()));
                if path_components.len() > 1 {
                    if let Some(parent_p) = import_path.parent() {
                        possible_file_paths.push(start_path.join(parent_p));
                    }
                }
            }
            possible_file_paths.iter().unique().map(|x| {
                let mut x = x.clone();
                x.set_extension("rs");
                x.canonicalize().unwrap_or(x)
            }).collect::<Vec<_>>()
        }
        LanguageId::TypeScript => {
            let import_path = path_components.iter().collect::<PathBuf>();
            let mut possible_file_paths = vec![];
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(prefix_p.join(import_path.clone()));
                if let Some(parent_p) = import_path.parent() {
                    possible_file_paths.push(prefix_p.join(parent_p));
                }
            }
            let mut start_path = file_path.clone();
            loop {
                if let Some(parent_p) = start_path.parent() {
                    start_path = parent_p.to_path_buf();
                    if min_prefix.clone().map(|x| x == start_path).unwrap_or(false) {
                        break;
                    }
                } else {
                    break;
                }
                possible_file_paths.push(start_path.join(import_path.clone()));
                if let Some(parent_p) = import_path.parent() {
                    possible_file_paths.push(start_path.join(parent_p));
                }
            }
            possible_file_paths.iter().unique().map(|x| {
                let mut x = x.clone();
                x.set_extension("ts");
                x.canonicalize().unwrap_or(x)
            }).collect::<Vec<_>>()
        }
        _ => {
            info!("unsupported language {} while resolving the import", language);
            vec![]
        }
    }
}