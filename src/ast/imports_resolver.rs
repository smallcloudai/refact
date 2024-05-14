use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceArc;
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


pub struct SearchItem {
    pub path: PathBuf,
    pub nameless_path: Option<PathBuf>,
    pub name: Option<String>,
    pub do_fuzzy_search: bool,
}

pub struct SearchResult {
    pub path: PathBuf,
    pub name: Option<String>,

}

pub fn try_find_file_path(
    search_items: &Vec<SearchItem>,
    path_by_symbols: &HashMap<PathBuf, Vec<AstSymbolInstanceArc>>,
    paths_str: &Vec<String>,
) -> Option<SearchResult> {
    for item in search_items {
        if path_by_symbols.contains_key(&item.path) {
            return Some(SearchResult {
                path: item.path.clone(),
                name: None,
            });
        } else if let Some(nameless_path) = &item.nameless_path {
            if path_by_symbols.contains_key(nameless_path) {
                return Some(SearchResult {
                    path: nameless_path.clone(),
                    name: item.name.clone(),
                });
            }
        }
    }

    // no exact match, try to found by path inclusion (could be slow, O(n))
    // in theory, this should be happened only if we not sure about the file's extension
    let items_to_fuzzy_search = search_items
        .iter()
        .filter(|item| item.do_fuzzy_search)
        .map(|x| (
            x.path.to_str().unwrap_or_default().to_string(),
            x.nameless_path.clone().unwrap_or_default().to_str().unwrap_or_default().to_string(),
            x))
        .collect::<Vec<_>>();
    for file_path in paths_str.iter() {
        for (path, nameless_path, item) in items_to_fuzzy_search.iter() {
            if file_path.contains(path) {
                return Some(SearchResult {
                    path: item.path.clone(),
                    name: None,
                });
            } else if file_path.contains(nameless_path) {
                if let Some(nameless_path) = &item.nameless_path {
                    return Some(SearchResult {
                        path: nameless_path.clone(),
                        name: item.name.clone(),
                    });
                }
            }
        }
    }
    None
}


fn prefixes_based_imports_retrieval(
    prefixes: &Vec<(PathBuf, usize)>,
    min_prefix: &Option<PathBuf>,
    file_path: &PathBuf,
    path_components: &Vec<String>,
    extension: &str,
) -> Vec<SearchItem> {
    let canonicalize_path = |path: &PathBuf| -> PathBuf {
        let ext_path = path.with_extension(extension);
        ext_path.canonicalize().unwrap_or(ext_path)
    };

    let import_path = path_components.iter().collect::<PathBuf>();
    let mut possible_file_paths = vec![];
    for (prefix_p, _) in prefixes.iter() {
        let (name, nameless_path) = if let Some(parent_p) = import_path.parent() {
            (import_path.file_name()
                 .map(|x| x.to_str())
                 .map(|x| x.map(|xx| xx.to_string()))
                 .flatten(),
             Some(prefix_p.join(parent_p)))
        } else {
            (None, None)
        };
        possible_file_paths.push(SearchItem {
            path: canonicalize_path(&prefix_p.join(import_path.clone())),
            nameless_path: nameless_path.map(|x| canonicalize_path(&x)),
            name,
            do_fuzzy_search: false,
        });
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
        let (name, nameless_path) = if let Some(parent_p) = import_path.parent() {
            (import_path.file_name()
                 .map(|x| x.to_str())
                 .map(|x| x.map(|xx| xx.to_string()))
                 .flatten(),
             Some(start_path.join(parent_p)))
        } else {
            (None, None)
        };
        possible_file_paths.push(SearchItem {
            path: canonicalize_path(&start_path.join(import_path.clone())),
            nameless_path: nameless_path.map(|x| canonicalize_path(&x)),
            name,
            do_fuzzy_search: false,
        });
    }
    possible_file_paths
}


pub fn possible_filepath_candidates(
    prefixes: &Vec<(PathBuf, usize)>,
    min_prefix: &Option<PathBuf>,
    file_path: &PathBuf,
    path_components: &Vec<String>,
    language: &LanguageId,
) -> Vec<SearchItem> {
    match language {
        LanguageId::Cpp => {
            let mut possible_file_paths = vec![];
            let import_path = path_components.iter().collect::<PathBuf>();
            let mut path = combine_paths(&file_path, &import_path);
            path = path.canonicalize().unwrap_or(path.clone());
            possible_file_paths.push(SearchItem {
                path,
                nameless_path: None,
                name: None,
                do_fuzzy_search: false,
            });
            possible_file_paths.push(SearchItem {
                path: relative_combine_paths(&file_path, &import_path),
                nameless_path: None,
                name: None,
                do_fuzzy_search: false,
            });
            possible_file_paths.push(SearchItem {
                path: import_path.canonicalize().unwrap_or(import_path.clone()),
                nameless_path: None,
                name: None,
                do_fuzzy_search: false,
            });
            for (prefix_p, _) in prefixes.iter() {
                possible_file_paths.push(SearchItem {
                    path: prefix_p.join(import_path.clone()),
                    nameless_path: None,
                    name: None,
                    do_fuzzy_search: false,
                });
            }
            possible_file_paths
        }
        LanguageId::JavaScript => {
            prefixes_based_imports_retrieval(
                prefixes,
                min_prefix,
                file_path,
                path_components,
                "js",
            )
        }
        LanguageId::Python => {
            prefixes_based_imports_retrieval(
                prefixes,
                min_prefix,
                file_path,
                path_components,
                "py",
            )
        }
        LanguageId::Rust => {
            prefixes_based_imports_retrieval(
                prefixes,
                min_prefix,
                file_path,
                path_components,
                "rs",
            )
        }
        LanguageId::TypeScript => {
            prefixes_based_imports_retrieval(
                prefixes,
                min_prefix,
                file_path,
                path_components,
                "ts",
            )
        }
        _ => {
            vec![]
        }
    }
}