use std::collections::{HashMap, HashSet};
use std::path::{Component, PathBuf};
use std::sync::Arc;
use itertools::Itertools;
use tokio::sync::RwLock as ARwLock;
use strsim::normalized_damerau_levenshtein;
use tracing::info;

use crate::global_context::GlobalContext;


fn make_cache<I>(paths_iter: I) -> (
    HashMap<String, HashSet<String>>, Vec<String>, usize
) where I: IntoIterator<Item = PathBuf> {
    let mut cache_correction = HashMap::<String, HashSet<String>>::new();
    let mut cache_fuzzy_set = HashSet::<String>::new();
    let mut cnt = 0;

    for path in paths_iter {
        let path_str = path.to_str().unwrap_or_default().to_string();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        cache_fuzzy_set.insert(file_name);
        cnt += 1;

        cache_correction.entry(path_str.clone()).or_insert_with(HashSet::new).insert(path_str.clone());
        // chop off directory names one by one
        let mut index = 0;
        while let Some(slashpos) = path_str[index .. ].find(|c| c == '/' || c == '\\') {
            let absolute_slashpos = index + slashpos;
            index = absolute_slashpos + 1;
            let slashpos_to_end = &path_str[index .. ];
            if !slashpos_to_end.is_empty() {
                cache_correction.entry(slashpos_to_end.to_string()).or_insert_with(HashSet::new).insert(path_str.clone());
            }
        }
    }

    (cache_correction, cache_fuzzy_set.into_iter().collect(), cnt)
}

use std::time::Instant;

pub async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>) -> (Arc<HashMap<String, HashSet<String>>>, Arc<Vec<String>>) {
    let (cache_dirty_arc, mut cache_correction_arc, mut cache_fuzzy_arc) = {
        let cx = global_context.read().await;
        (
            cx.documents_state.cache_dirty.clone(),
            cx.documents_state.cache_correction.clone(),
            cx.documents_state.cache_fuzzy.clone(),
        )
    };

    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref {
        info!("Rebuilding files cache...");
        let start_time = Instant::now();

        let (file_paths_from_memory, paths_from_workspace, paths_from_jsonl) = {
            let cx = global_context.read().await;
            let memory_docs = cx.documents_state.memory_document_map.keys().cloned().collect::<Vec<_>>();
            let workspace_files = cx.documents_state.workspace_files.lock().unwrap().clone();
            let jsonl_files = cx.documents_state.jsonl_files.lock().unwrap().clone();
            (memory_docs, workspace_files, jsonl_files)
        };

        let paths_from_anywhere = file_paths_from_memory.into_iter().chain(paths_from_workspace.into_iter().chain(paths_from_jsonl.into_iter()));
        let (cache_correction, cache_fuzzy, cnt) = make_cache(paths_from_anywhere);

        info!("Rebuild completed in {}s, {} URLs => cache_correction.len is now {}", start_time.elapsed().as_secs(), cnt, cache_correction.len());

        cache_correction_arc = Arc::new(cache_correction);
        cache_fuzzy_arc = Arc::new(cache_fuzzy);
        {
            let mut cx = global_context.write().await;
            cx.documents_state.cache_correction = cache_correction_arc.clone();
            cx.documents_state.cache_fuzzy = cache_fuzzy_arc.clone();
        }
        *cache_dirty_ref = false;
    }

    return (cache_correction_arc, cache_fuzzy_arc);
}

pub async fn correct_to_nearest_filename(
    global_context: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    let (cache_correction_arc, cache_fuzzy_arc) = files_cache_rebuild_as_needed(global_context.clone()).await;
    // it's dangerous to use cache_correction_arc without a mutex, but should be fine as long as it's read-only
    // (another thread never writes to the map itself, it can only replace the arc with a different map)

    if let Some(fixed) = (*cache_correction_arc).get(&correction_candidate.clone()) {
        // info!("found {:?} in cache_correction, returning [{:?}]", correction_candidate, fixed);
        return fixed.into_iter().cloned().collect::<Vec<String>>();
    } else {
        info!("not found {} in cache_correction", correction_candidate);
    }

    if fuzzy {
        info!("fuzzy search {:?}, cache_fuzzy_arc.len={}", correction_candidate, cache_fuzzy_arc.len());
        let mut top_n_records: Vec<(String, f64)> = Vec::with_capacity(top_n);
        for p in cache_fuzzy_arc.iter() {
            let dist = normalized_damerau_levenshtein(&correction_candidate, p);
            top_n_records.push((p.clone(), dist));
            if top_n_records.len() >= top_n {
                top_n_records.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                top_n_records.pop();
            }
        }
        info!("the top{} nearest matches {:?}", top_n, top_n_records);
        let mut sorted_paths: Vec<String> = vec![];
        for path in top_n_records.iter().sorted_by(|a, b|a.1.partial_cmp(&b.1).unwrap()).rev().map(|(path, _)| path) {
            if let Some(fixed) = (*cache_correction_arc).get(path) {
                sorted_paths.extend(fixed.into_iter().cloned());
            } else {
                sorted_paths.push(path.clone());
            }
        }
        return sorted_paths;
    }

    return vec![];
}

fn absolute(path: &std::path::Path) -> std::io::Result<PathBuf> {
    let mut components = path.strip_prefix(".").unwrap_or(path).components();
    let path_os = path.as_os_str().as_encoded_bytes();
    let mut normalized = if path.is_absolute() {
        if path_os.starts_with(b"//") && !path_os.starts_with(b"///") {
            components.next();
            PathBuf::from("//")
        } else {
            PathBuf::new()
        }
    } else {
        std::env::current_dir()?
    };
    normalized.extend(components);
    if path_os.ends_with(b"/") {
        normalized.push("");
    }
    Ok(normalized)
}

pub fn canonical_path(s: &String) -> PathBuf {
    let mut res = match PathBuf::from(s).canonicalize() {
        Ok(x) => x,
        Err(_) => {
            let a = absolute(std::path::Path::new(s)).unwrap_or(PathBuf::from(s));
            // warn!("canonical_path: {:?} doesn't work: {}\n using absolute path instead {}", s, e, a.display());
            a
        }
    };
    let components: Vec<String> = res
        .components()
        .map(|x| match x {
            Component::Normal(c) => c.to_string_lossy().to_string(),
            Component::Prefix(c) => {
                let lowercase_prefix = c.as_os_str().to_string_lossy().to_string().to_lowercase();
                lowercase_prefix
            },
            _ => x.as_os_str().to_string_lossy().to_string(),
        })
        .collect();
    res = components.iter().fold(PathBuf::new(), |mut acc, x| {
        acc.push(x);
        acc
    });
    // info!("canonical_path:\n{:?}\n{:?}", s, res);
    res
}

