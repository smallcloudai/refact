use std::collections::{HashMap, HashSet};
use std::path::{Component, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context::GlobalContext;

pub async fn paths_from_anywhere(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let file_paths_from_memory = global_context.read().await.documents_state.memory_document_map.keys().map(|x|x.clone()).collect::<Vec<_>>();
    let paths_from_workspace: Vec<PathBuf> = global_context.read().await.documents_state.workspace_files.lock().unwrap().clone();
    let paths_from_jsonl: Vec<PathBuf> = global_context.read().await.documents_state.jsonl_files.lock().unwrap().clone();
    let paths_from_anywhere = file_paths_from_memory.into_iter().chain(paths_from_workspace.into_iter().chain(paths_from_jsonl.into_iter()));
    paths_from_anywhere.collect::<Vec<PathBuf>>()
}

fn make_cache(paths: &Vec<PathBuf>, workspace_folders: &Vec<PathBuf>) -> (
    HashMap<String, HashSet<String>>, HashSet<String>, usize
) {
    let mut cache_correction = HashMap::<String, HashSet<String>>::new();
    let mut cnt = 0;

    for path in paths {
        let path_str = path.to_str().unwrap_or_default().to_string();

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

    // Find the shortest unique suffix for each path, that is at least the path from workspace root
    let cache_shortened: HashSet<String> = paths.iter().map(|path| {
        let workspace_components_len = workspace_folders.iter()
            .filter_map(|workspace_dir| {
                if path.starts_with(workspace_dir) {
                    Some(workspace_dir.components().count())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        let path_is_dir = path.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR);
        let mut current_suffix = PathBuf::new();
        let path_components_count = path.components().count();
        for component in path.components().rev() {
            if !current_suffix.as_os_str().is_empty() || path_is_dir {
                current_suffix = PathBuf::from(component.as_os_str()).join(&current_suffix);
            } else {
                current_suffix = PathBuf::from(component.as_os_str());
            }
            let suffix = current_suffix.to_string_lossy().into_owned();
            if cache_correction.get(suffix.as_str()).map_or(0, |v| v.len()) == 1 &&
                current_suffix.components().count() + workspace_components_len >= path_components_count {
                cnt += 1;
                return suffix;
            }
        }
        cnt += 1;
        path.to_string_lossy().into_owned()
    }).collect();

    (cache_correction, cache_shortened, cnt)
}

pub async fn get_files_in_dir(
    global_context: Arc<ARwLock<GlobalContext>>,
    dir: &PathBuf,
) -> Vec<PathBuf> {
    let paths = paths_from_anywhere(global_context.clone()).await;
    paths.into_iter()
        .filter(|path| path.parent() == Some(dir))
        .collect()
}

pub async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>) -> (Arc<HashMap<String, HashSet<String>>>, Arc<HashSet<String>>) {
    let (cache_dirty_arc, mut cache_correction_arc, mut cache_shortened_arc) = {
        let cx = global_context.read().await;
        (
            cx.documents_state.cache_dirty.clone(),
            cx.documents_state.cache_correction.clone(),
            cx.documents_state.cache_shortened.clone(),
        )
    };

    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref {
        info!("rebuilding files cache...");
        // filter only get_project_dirs?
        let start_time = Instant::now();
        let paths_from_anywhere = paths_from_anywhere(global_context.clone()).await;
        let workspace_folders = get_project_dirs(global_context.clone()).await;
        let (cache_correction, cache_shortened, cnt) = make_cache(&paths_from_anywhere, &workspace_folders);

        info!("rebuild completed in {:.3}s, {} URLs => cache_correction.len is now {}", start_time.elapsed().as_secs_f64(), cnt, cache_correction.len());
        cache_correction_arc = Arc::new(cache_correction);
        cache_shortened_arc = Arc::new(cache_shortened);
        {
            let mut cx = global_context.write().await;
            cx.documents_state.cache_correction = cache_correction_arc.clone();
            cx.documents_state.cache_shortened = cache_shortened_arc.clone();
        }
        *cache_dirty_ref = false;
    }

    return (cache_correction_arc, cache_shortened_arc);
}

pub fn fuzzy_search<I>(
    correction_candidate: &String,
    candidates: I,
    top_n: usize,
) -> Vec<String>
where I: IntoIterator<Item = String> {
    const FILENAME_WEIGHT: i32 = 3;
    const DISTANCE_THRESHOLD: f64 = 0.45;
    const EXCESS_WEIGHT: f64 = 3.0;

    let mut correction_bigram_count: HashMap<(char, char), i32> = HashMap::new();

    // Count bigrams of correction candidate
    let mut correction_candidate_length = 0;
    let mut weight = FILENAME_WEIGHT;
    for window in correction_candidate.to_lowercase().chars().collect::<Vec<_>>().windows(2).rev() {
        if window[0] == std::path::MAIN_SEPARATOR {
            weight = 1;
        }
        correction_candidate_length += weight;
        *correction_bigram_count
            .entry((window[0], window[1]))
            .or_insert(0) += weight;
    }

    let mut top_n_candidates = Vec::new();

    for candidate in candidates {
        let mut missing_count: i32 = 0;
        let mut excess_count = 0;
        let mut candidate_len = 0;
        let mut bigram_count = correction_bigram_count.clone();

        // Discount candidate's bigrams from correction candidate's ones
        let mut weight = FILENAME_WEIGHT;
        for window in candidate.to_lowercase().chars().collect::<Vec<_>>().windows(2).rev() {
            if window[0] == std::path::MAIN_SEPARATOR {
                weight = 1;
            }
            candidate_len += weight;
            if let Some(entry) = bigram_count.get_mut(&(window[0], window[1])) {
                *entry -= weight;
            } else {
                missing_count += weight;
            }
        }

        for (&_, &count) in bigram_count.iter() {
            if count > 0 {
                excess_count += count;
            } else {
                missing_count += -count;
            }
        }

        let distance = (missing_count as f64 + excess_count as f64 * EXCESS_WEIGHT) /
            (correction_candidate_length as f64 + (candidate_len as f64) * EXCESS_WEIGHT);
        if distance < DISTANCE_THRESHOLD {
            top_n_candidates.push((candidate, distance));
            top_n_candidates
                .sort_by(|a, b| a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal));
            if top_n_candidates.len() > top_n {
                top_n_candidates.pop();
            }
        }
    }

    top_n_candidates.into_iter().map(|x| x.0).collect()
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
        return fuzzy_search(correction_candidate, cache_fuzzy_arc.iter().cloned(), top_n);
    }

    return vec![];
}

pub async fn correct_to_nearest_dir_path(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    fn get_parent(p: &String) -> Option<String> {
        PathBuf::from(p).parent().map(PathBuf::from).map(|x|x.to_string_lossy().to_string())
    }

    let (cache_correction_arc, cache_fuzzy_set) = files_cache_rebuild_as_needed(gcx.clone()).await;
    let mut paths_correction_map = HashMap::new();
    for (k, v) in cache_correction_arc.iter() {
        match get_parent(k) {
            Some(k_parent) => {
                let v_parents = v.iter().filter_map(|x| get_parent(x)).collect::<Vec<_>>();
                if v_parents.is_empty() {
                    continue;
                }
                paths_correction_map.entry(k_parent.clone()).or_insert_with(HashSet::new).extend(v_parents);
            },
            None => {}
        }
    }
    if let Some(res) = paths_correction_map.get(correction_candidate).map(|x|x.iter().cloned().collect::<Vec<_>>()) {
        return res;
    }

    if fuzzy {
        let mut dirs = HashSet::<String>::new();

        for p in cache_fuzzy_set.iter() {
            let mut current_path = PathBuf::from(&p);
            while let Some(parent) = current_path.parent() {
                dirs.insert(parent.to_string_lossy().to_string());
                current_path = parent.to_path_buf();
            }
        }

        info!("fuzzy search {:?}, dirs.len={}", correction_candidate, dirs.len());
        return fuzzy_search(correction_candidate, dirs.iter().cloned(), top_n);
    }
    vec![]
}

pub async fn get_project_dirs(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let gcx_locked = gcx.write().await;
    let workspace_folders = gcx_locked.documents_state.workspace_folders.lock().unwrap();
    workspace_folders.iter().cloned().collect::<Vec<_>>()
}

pub async fn shortify_paths(gcx: Arc<ARwLock<GlobalContext>>, paths: Vec<String>) -> Vec<String> {
    let (_, indexed_paths) = files_cache_rebuild_as_needed(gcx.clone()).await;
    let workspace_folders = get_project_dirs(gcx.clone()).await
        .iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>();
    shortify_paths_from_indexed(paths, indexed_paths, workspace_folders)
}

fn shortify_paths_from_indexed(paths: Vec<String>, indexed_paths: Arc<HashSet<String>>, workspace_folders: Vec<String>) -> Vec<String> {
    paths.into_iter().map(|mut path| {
        // Get the length of the workspace part of the path
        let workspace_part_len = workspace_folders.iter()
            .filter_map(|workspace_dir| {
                if path.starts_with(workspace_dir) {
                    Some(workspace_dir.len())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        // Find the longest suffix of the path, that is in the indexed cache, make sure it is at
        // least as long as the part of the path relative to the workspace root
        let full_path = path.clone();
        while !path.is_empty() {
            if indexed_paths.get(&path).is_some() &&
                workspace_part_len + if std::path::MAIN_SEPARATOR == '/' { 1 } else { 2 } + path.len() >= full_path.len() {
                return path;
            }
            path.drain(..1);
        }
        full_path
    }).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_in_workspace::retrieve_files_in_workspace_folders;

    async fn get_candidates_from_workspace_files() -> Vec<String> {
        let proj_folders = vec![PathBuf::from(".").canonicalize().unwrap()];
        let proj_folder = &proj_folders[0];

        let workspace_files = retrieve_files_in_workspace_folders(proj_folders.clone()).await;

        workspace_files
            .iter()
            .filter_map(|path| {
                let relative_path = path.strip_prefix(proj_folder)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                    Some(relative_path)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_fuzzy_search_finds_frog_py() {
        // Arrange
        let correction_candidate = "frog.p".to_string();
        let top_n = 1;

        let candidates = get_candidates_from_workspace_files().await;

        // Act
        let result = fuzzy_search(&correction_candidate, candidates, top_n);

        // Assert
        let expected_result = vec![
            PathBuf::from("tests").join("emergency_frog_situation").join("frog.py").to_string_lossy().to_string(),
        ];

        assert_eq!(result, expected_result, "It should find the proper frog.py, found {:?} instead", result);
    }

    #[tokio::test]
    async fn test_fuzzy_search_path_helps_finding_file() {
        // Arrange
        let correction_candidate = PathBuf::from("emergency_frog_situation").join("wo").to_string_lossy().to_string();
        let top_n = 1;

        let candidates = get_candidates_from_workspace_files().await;

        // Act
        let result = fuzzy_search(&correction_candidate, candidates, top_n);

        // Assert
        let expected_result = vec![
            PathBuf::from("tests").join("emergency_frog_situation").join("work_day.py").to_string_lossy().to_string(),
        ];

        assert_eq!(result, expected_result, "It should find the proper file (work_day.py), found {:?} instead", result);
    }

    #[tokio::test]
    async fn test_fuzzy_search_filename_weights_more_than_path() {
        // Arrange
        let correction_candidate = "my_file.ext".to_string();
        let top_n = 2;

        let candidates = vec![
            PathBuf::from("my_library").join("implementation").join("my_file.ext").to_string_lossy().to_string(),
            PathBuf::from("my_library").join("my_file.ext").to_string_lossy().to_string(),
            PathBuf::from("another_file.ext").to_string_lossy().to_string(),
        ];

        // Act
        let result = fuzzy_search(&correction_candidate, candidates, top_n);

        // Assert
        let expected_result = vec![
            PathBuf::from("my_library").join("my_file.ext").to_string_lossy().to_string(),
            PathBuf::from("my_library").join("implementation").join("my_file.ext").to_string_lossy().to_string(),
        ];

        let mut sorted_result = result.clone();
        let mut sorted_expected = expected_result.clone();

        sorted_result.sort();
        sorted_expected.sort();

        assert_eq!(sorted_result, sorted_expected, "The result should contain the expected paths in any order, found {:?} instead", result);
    }

    #[test]
    fn test_make_cache() {
        // Arrange
        let paths = vec![
            PathBuf::from("home").join("user").join("repo1").join("dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo2").join("dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo1").join("this_file.ext"),
            PathBuf::from("home").join("user").join("repo2").join("dir").join("this_file.ext"),
            PathBuf::from("home").join("user").join("repo2").join("dir2"),
        ];

        let workspace_folders = vec![
            PathBuf::from("home").join("user").join("repo1"),
            PathBuf::from("home").join("user").join("repo2"),
        ];

        // Act
        let (_, cache_shortened_result, cnt) = make_cache(&paths, &workspace_folders);

        // Assert
        let mut cache_shortened_result_vec = cache_shortened_result.into_iter().collect::<Vec<_>>();
        let mut expected_result = vec![
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo1").join("this_file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir").join("this_file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").to_string_lossy().to_string(),
        ];

        expected_result.sort();
        cache_shortened_result_vec.sort();

        assert_eq!(cnt, 5, "The cache should contain 5 paths");
        assert_eq!(cache_shortened_result_vec, expected_result, "The result should contain the expected paths, instead it found");
    }

    #[test]
    fn test_shortify_paths_from_indexed() {
        let workspace_folders = vec![
            PathBuf::from("home").join("user").join("repo1").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join("nested").join("repo2").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo3").to_string_lossy().to_string(),
        ];

        let indexed_paths = Arc::new(HashSet::from([
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo1").join("this_file.ext").to_string_lossy().to_string(),
            PathBuf::from("custom_dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").join("another_file.ext").to_string_lossy().to_string(),
        ]));

        let paths = vec![
            PathBuf::from("home").join("user").join("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join("nested").join("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join(".hidden").join("custom_dir").join("file.ext").to_string_lossy().to_string(), 
            // Hidden file; should not be shortened as it's not in the cache and may be confused with custom_dir/file.ext.
            PathBuf::from("home").join("user").join("repo3").join("dir2").join("another_file.ext").to_string_lossy().to_string(),
        ];

        let result = shortify_paths_from_indexed(paths, indexed_paths, workspace_folders);

        let expected_result = vec![
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join(".hidden").join("custom_dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").join("another_file.ext").to_string_lossy().to_string(),
        ];

        assert_eq!(result, expected_result, "The result should contain the expected paths, instead it found");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_make_cache_speed() {
        // Arrange
        let workspace_paths = vec![
            PathBuf::from("home").join("user").join("repo1"),
            PathBuf::from("home").join("user").join("repo2"),
            PathBuf::from("home").join("user").join("repo3"),
            PathBuf::from("home").join("user").join("repo4"),
        ];

        let mut paths = Vec::new();
        for i in 0..100000 {
            let path = workspace_paths[i % workspace_paths.len()]
                .join(format!("dir{}", i % 1000))
                .join(format!("dir{}", i / 1000))
                .join(format!("file{}.ext", i));
            paths.push(path);
        }
        let start_time = std::time::Instant::now();

        // Act
        let (_, cache_shortened_result, cnt) = make_cache(&paths, &workspace_paths);

        // Assert
        let time_spent = start_time.elapsed();
        println!("make_cache took {} ms", time_spent.as_millis());
        assert!(time_spent.as_millis() < 2500, "make_cache took {} ms", time_spent.as_millis());

        assert_eq!(cnt, 100000, "The cache should contain 100000 paths");
        assert_eq!(cache_shortened_result.len(), cnt);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_fuzzy_search_speed() {
        // Arrange
        let workspace_paths = vec![
            PathBuf::from("home").join("user").join("repo1"),
            PathBuf::from("home").join("user").join("repo2"),
            PathBuf::from("home").join("user").join("repo3"),
            PathBuf::from("home").join("user").join("repo4"),
        ];

        let mut paths = Vec::new();
        for i in 0..100000 {
            let path = workspace_paths[i % workspace_paths.len()]
                .join(format!("dir{}", i % 1000))
                .join(format!("dir{}", i / 1000))
                .join(format!("file{}.ext", i));
            paths.push(path);
        }
        let start_time = std::time::Instant::now();
        let paths_str = paths.iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>();

        let correction_candidate = PathBuf::from("file100000")
            .join("dir1000")
            .join("file100000.ext")
            .to_string_lossy()
            .to_string();

        // Act
        let results = fuzzy_search(&correction_candidate, paths_str, 10);

        // Assert
        let time_spent = start_time.elapsed();
        println!("fuzzy_search took {} ms", time_spent.as_millis());
        assert!(time_spent.as_millis() < 750, "fuzzy_search took {} ms", time_spent.as_millis());

        assert_eq!(results.len(), 10, "The result should contain 10 paths");
        println!("{:?}", results);
    }
}