use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use std::path::{PathBuf, Component, Path};
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context::GlobalContext;
use crate::custom_error::MapErrToString;
use crate::files_in_workspace::detect_vcs_for_a_file_path;
use crate::fuzzy_search::fuzzy_search;


pub async fn paths_from_anywhere(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let (file_paths_from_memory, paths_from_workspace, paths_from_jsonl) = {
        let documents_state = &global_context.read().await.documents_state;  // somehow keeps lock until out of scope
        let file_paths_from_memory = documents_state.memory_document_map.keys().cloned().collect::<Vec<_>>();
        let paths_from_workspace = documents_state.workspace_files.lock().unwrap().clone();
        let paths_from_jsonl = documents_state.jsonl_files.lock().unwrap().clone();
        (file_paths_from_memory, paths_from_workspace, paths_from_jsonl)
    };

    let paths_from_anywhere = file_paths_from_memory
        .into_iter()
        .chain(paths_from_workspace.into_iter().chain(paths_from_jsonl.into_iter()));

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

pub async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>) -> (Arc<HashMap<String, HashSet<String>>>, Arc<HashSet<String>>) {
    let (cache_dirty_arc, mut cache_correction_arc, mut cache_shortened_arc) = {
        let cx = global_context.read().await;
        (
            cx.documents_state.cache_dirty.clone(),
            cx.documents_state.cache_correction.clone(),
            cx.documents_state.cache_shortened.clone(),
        )
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref > 0.0 && now > *cache_dirty_ref {
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
        *cache_dirty_ref = 0.0;
    }

    return (cache_correction_arc, cache_shortened_arc);
}

async fn complete_path_with_project_dir(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    is_dir: bool,
) -> Option<PathBuf> {
    fn path_exists(path: &PathBuf, is_dir: bool) -> bool {
        (is_dir && path.is_dir()) || (!is_dir && path.is_file())
    }
    let candidate_path = canonical_path(correction_candidate);
    let project_dirs = get_project_dirs(gcx.clone()).await;
    for p in project_dirs {
        if path_exists(&candidate_path, is_dir) && candidate_path.starts_with(&p) {
            return Some(candidate_path);
        }
        let j_path = p.join(&candidate_path);
        if path_exists(&j_path, is_dir) {
            return Some(j_path);
        }

        // This might save a roundtrip:
        // .../project1/project1/1.cpp
        // model likes to output only one "project1" of the two needed
        if candidate_path.starts_with(&p) {
            let last_component = p.components()
                .last()
                .map(|x| x.as_os_str().to_string_lossy().to_string())
                .unwrap_or("".to_string());
            let last_component_duplicated = p
                .join(&last_component)
                .join(&candidate_path.strip_prefix(&p).unwrap_or(candidate_path.as_path()));
            if path_exists(&last_component_duplicated, is_dir) {
                info!(
                    "autocorrected by duplicating the project last component: {} -> {}",
                    p.to_string_lossy().to_string(),
                    last_component_duplicated.to_string_lossy().to_string()
                );
                return Some(last_component_duplicated);
            }
        }
    }
    None
}

pub async fn correct_to_nearest_filename(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    if let Some(fixed) = complete_path_with_project_dir(gcx.clone(), correction_candidate, false).await {
        return vec![fixed.to_string_lossy().to_string()];
    }

    let (cache_correction_arc, cache_fuzzy_arc) = files_cache_rebuild_as_needed(gcx.clone()).await;
    // it's dangerous to use cache_correction_arc without a mutex, but should be fine as long as it's read-only
    // (another thread never writes to the map itself, it can only replace the arc with a different map)

    if let Some(fixed) = (*cache_correction_arc).get(&correction_candidate.clone()) {
        return fixed.into_iter().cloned().collect::<Vec<String>>();
    } else {
        info!("not found {:?} in cache_correction", correction_candidate);
    }

    if fuzzy {
        info!("fuzzy search {:?}, cache_fuzzy_arc.len={}", correction_candidate, cache_fuzzy_arc.len());
        return fuzzy_search(correction_candidate, cache_fuzzy_arc.iter().cloned(), top_n, &['/', '\\']);
    }

    return vec![];
}

pub async fn correct_to_nearest_dir_path(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    if let Some(fixed) = complete_path_with_project_dir(gcx.clone(), correction_candidate, true).await {
        return vec![fixed.to_string_lossy().to_string()];
    }

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
        return fuzzy_search(correction_candidate, dirs.iter().cloned(), top_n, &['/', '\\']);
    }
    vec![]
}

pub async fn get_project_dirs(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let workspace_folders = gcx.read().await.documents_state.workspace_folders.clone();
    let workspace_folders_locked = workspace_folders.lock().unwrap();
    workspace_folders_locked.iter().cloned().collect::<Vec<_>>()
}

pub async fn get_active_project_path(gcx: Arc<ARwLock<GlobalContext>>) -> Option<PathBuf> {
    let workspace_folders = get_project_dirs(gcx.clone()).await;
    if workspace_folders.is_empty() { return None; }

    let active_file = gcx.read().await.documents_state.active_file_path.clone();
    // tracing::info!("get_active_project_path(), active_file={:?} workspace_folders={:?}", active_file, workspace_folders);

    let active_file_path = if let Some(active_file) = active_file {
        active_file
    } else {
        // tracing::info!("returning the first workspace folder: {:?}", workspace_folders[0]);
        return Some(workspace_folders[0].clone());
    };

    if let Some((path, _)) = detect_vcs_for_a_file_path(&active_file_path).await {
        // tracing::info!("found VCS path: {:?}", path);
        return Some(path);
    }

    // Without VCS, return one of workspace_folders that is a parent for active_file_path
    for f in workspace_folders {
        if active_file_path.starts_with(&f) {
            // tracing::info!("found that {:?} is the workspace folder", f);
            return Some(f);
        }
    }

    tracing::info!("no project is active");
    None
}

pub async fn get_active_workspace_folder(gcx: Arc<ARwLock<GlobalContext>>) -> Option<PathBuf> {
    let workspace_folders = get_project_dirs(gcx.clone()).await;

    let active_file = gcx.read().await.documents_state.active_file_path.clone();
    if let Some(active_file) = active_file {
        for f in &workspace_folders {
            if active_file.starts_with(f) {
                tracing::info!("found that {:?} is the workspace folder", f);
                return Some(f.clone());
            }
        }
    }

    if let Some(first_workspace_folder) = workspace_folders.first() {
        tracing::info!("found that {:?} is the workspace folder", first_workspace_folder);
        Some(first_workspace_folder.clone())
    } else {
        None
    }
}

pub async fn shortify_paths(gcx: Arc<ARwLock<GlobalContext>>, paths: &Vec<String>) -> Vec<String> {
    let (_, indexed_paths) = files_cache_rebuild_as_needed(gcx.clone()).await;
    let workspace_folders = get_project_dirs(gcx.clone()).await
        .iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>();
    _shortify_paths_from_indexed(paths, indexed_paths, workspace_folders)
}

fn _shortify_paths_from_indexed(paths: &Vec<String>, indexed_paths: Arc<HashSet<String>>, workspace_folders: Vec<String>) -> Vec<String>
{
    paths.into_iter().map(|path| {
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
        let mut path_to_cut = path.clone();
        while !path_to_cut.is_empty() {
            if indexed_paths.get(&path_to_cut).is_some() &&
                workspace_part_len + if std::path::MAIN_SEPARATOR == '/' { 1 } else { 2 } + path_to_cut.len() >= path.len() {
                return path_to_cut.clone();
            }
            path_to_cut.drain(..1);
        }
        path.clone()
    }).collect()
}

#[cfg(windows)]
/// In Windows, tries to fix the path, permissive about paths like \\?\C:\path, incorrect amount of \ and more.
///
/// Temporarily remove verbatim, to resolve ., .., symlinks if possible, it will be added again later.
pub fn preprocess_path_for_normalization(p: String) -> String {
    use itertools::Itertools;

    let p = p.replace(r"/", r"\");
    let starting_slashes = p.chars().take_while(|c| *c == '\\').count();

    let mut parts_iter = p.split(r"\").filter(|part| !part.is_empty()).peekable();

    match parts_iter.peek() {
        Some(&"?") => {
            parts_iter.next();
            match parts_iter.peek() {
                Some(pref) if pref.contains(":") => parts_iter.join(r"\"), // \\?\C:\path...
                Some(pref) if pref.to_lowercase() == "unc" => { // \\?\UNC\server\share\path...
                    parts_iter.next();
                    format!(r"\\{}", parts_iter.join(r"\"))
                },
                Some(_) => { // \\?\path...
                    tracing::warn!("Found a verbatim path that is not UNC nor Disk path: {}, leaving it as-is", p);
                    p
                },
                None => p, // \\?\
            }
        },
        Some(&".") if starting_slashes > 0 => {
            parts_iter.next();
            format!(r"\\.\{}", parts_iter.join(r"\")) // \\.\path...
        },
        Some(pref) if pref.contains(":") => parts_iter.join(r"\"), // C:\path...
        Some(_) => {
            match starting_slashes {
                0 => parts_iter.join(r"\"), // relative path: folder\file.ext
                1 => format!(r"\{}", parts_iter.join(r"\")), // absolute path from cur disk: \folder\file.ext
                _ => format!(r"\\{}", parts_iter.join(r"\")), // standard UNC path: \\server\share\folder\file.ext
            }
        }
        None => p, // \
    }
}

#[cfg(not(windows))]
/// In Unix, do nothing
pub fn preprocess_path_for_normalization(p: String) -> String {
    p
}

#[cfg(windows)]
/// In Windows, call std::path::absolute, then add verbatim prefix to standard Disk or UNC paths
fn absolute(path: &Path) -> Result<PathBuf, String> {
    use std::path::Prefix;
    use std::ffi::OsString;

    let path = std::path::absolute(path).map_err_to_string()?;

    if let Some(Component::Prefix(pref)) = path.components().next() {
        match pref.kind() {
            Prefix::Disk(_) => {
                let mut path_os_str = OsString::from(r"\\?\");
                path_os_str.push(path.as_os_str());
                Ok(PathBuf::from(path_os_str))
            },
            Prefix::UNC(_, _) => {
                let mut path_os_str = OsString::from(r"\\?\UNC\");
                path_os_str.push(path.strip_prefix(r"\\").unwrap_or(&path).as_os_str());
                Ok(PathBuf::from(path_os_str))
            },
            _ => Ok(path.to_path_buf())
        }
    } else {
        Ok(path.to_path_buf())
    }
}

#[cfg(not(windows))]
/// In Unix, this method is similar to std::path::absolute, but also resolves ..
fn absolute(path: &Path) -> Result<PathBuf, String> {
    let mut components = path.components();
    let path_os = path.as_os_str().as_encoded_bytes();

    let mut normalized = if path.is_absolute() {
        if path_os.starts_with(b"//") && !path_os.starts_with(b"///") {
            components.next();
            PathBuf::from("//")
        } else {
            PathBuf::from("/")
        }
    } else {
        std::env::current_dir().map_err_to_string()?
    };
    for component in components {
        match component {
            Component::Normal(c) => { normalized.push(c); }
            Component::ParentDir => { normalized.pop(); }
            Component::CurDir => (),
            Component::RootDir => (),
            Component::Prefix(_) => return Err("Prefix should not occur in Unix".to_string()),
        }
    }

    if path_os.ends_with(b"/") {
        normalized.push("");
    }

    Ok(normalized)
}

pub fn canonical_path<T: Into<String>>(p: T) -> PathBuf {
    let p: String = p.into();
    let path= PathBuf::from(preprocess_path_for_normalization(p));
    canonicalize_normalized_path(path)
}

/// If you did not call preprocess_path_for_normalization() before, use crate::files_correction::canonical_path() instead
pub fn canonicalize_normalized_path(p: PathBuf) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| absolute(&p).unwrap_or(p))
}

pub fn serialize_path<S: serde::Serializer>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}

pub fn deserialize_path<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    Ok(PathBuf::from(String::deserialize(deserializer)?))
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let result = _shortify_paths_from_indexed(&paths, indexed_paths, workspace_folders);

        let expected_result = vec![
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join(".hidden").join("custom_dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").join("another_file.ext").to_string_lossy().to_string(),
        ];

        assert_eq!(result, expected_result, "The result should contain the expected paths, instead it found");
    }

    #[cfg(windows)]
    #[test]
    fn test_preprocess_windows_path_for_normalization() {
        let test_cases = [
            // Verbatim disk paths
            (r"\\\\\\\\?\\\\C:\\\\Windows\\\\System32", r"C:\Windows\System32"),
            (r"\?\C:\Model generates this kind of paths", r"C:\Model generates this kind of paths"),
            (r"/?/C:/other\\horr.ible/path", r"C:\other\horr.ible\path"),

            // Disk paths
            (r"C:\\folder/..\\\\file", r"C:\folder\..\file"),
            (r"/D:\\Users/John Doe\\\\.\myfolder/file.ext", r"D:\Users\John Doe\.\myfolder\file.ext"),

            // Verbatim UNC paths
            (r"\\?\UNC\server\share/folder//file.ext", r"\\server\share\folder\file.ext"),
            (r"\\?\unc\server\share/folder//file.ext", r"\\server\share\folder\file.ext"),
            (r"/?/unc/server/share/folder//file.ext", r"\\server\share\folder\file.ext"),

            // Standard UNC paths
            (r"\\server\share/folder//file.ext", r"\\server\share\folder\file.ext"),
            (r"////server//share//folder//file.ext", r"\\server\share\folder\file.ext"),
            (r"//wsl$/Ubuntu/home/yourusername/projects", r"\\wsl$\Ubuntu\home\yourusername\projects"),

            // DeviceNS paths
            (r"////./pipe/docker_engine", r"\\.\pipe\docker_engine"),
            (r"\\.\pipe\docker_engine", r"\\.\pipe\docker_engine"),
            (r"//./pipe/docker_engine", r"\\.\pipe\docker_engine"),

            // Absolute paths without disk
            (r"\Windows\System32", r"\Windows\System32"),
            (r"/Program Files/Common Files", r"\Program Files\Common Files"),
            (r"\Users\Public\Downloads", r"\Users\Public\Downloads"),
            (r"\temp/path", r"\temp\path"),

            // Relative paths
            (r"folder/file.txt", r"folder\file.txt"),
            (r"./current/./folder", r".\current\.\folder"),
            (r"project/../src/main.rs", r"project\..\src\main.rs"),
            (r"documents\\photos", r"documents\photos"),
            (r"some folder/with spaces/file", r"some folder\with spaces\file"),
            (r"bin/../lib/./include", r"bin\..\lib\.\include"),
        ];

        for (input, expected) in test_cases {
            let result = preprocess_path_for_normalization(input.to_string());
            assert_eq!(result, expected.to_string(), "The result for {} should be {}, got {}", input, expected, result);
        }
    }

    #[cfg(windows)]
    #[ignore]
    #[test]
    fn test_canonical_path_windows()
    {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path();
        let temp_dir_path_str = temp_dir_path.to_str().unwrap();

        let long_str = String::from_utf8(vec![b'a'; 600].iter().map(|b| *b).collect()).unwrap();
        let long_dir_path = PathBuf::from(format!("\\\\?\\{temp_dir_path_str}\\{long_str}"));

        let create_dir_cmd = format!(
            "powershell.exe -Command \"New-Item -Path '{}' -ItemType Directory -Force\"",
            long_dir_path.to_string_lossy().replace("'", "''")
        );
        let create_file_cmd = format!(
            "powershell.exe -Command \"New-Item -Path '{}' -ItemType File -Force\"",
            long_dir_path.join("file.txt").to_string_lossy().replace("'", "''")
        );
        std::process::Command::new("cmd")
            .args(["/C", &create_dir_cmd])
            .output()
            .expect("Failed to create directory");
        std::process::Command::new("cmd")
            .args(["/C", &create_file_cmd])
            .output()
            .expect("Failed to create file");

        let long_dir_path_str = format!("{temp_dir_path_str}\\{long_str}\\..\\{long_str}");
        let long_dir_file_str = format!("{temp_dir_path_str}\\{long_str}\\..\\{long_str}\\.\\..\\{long_str}\\file.txt");

        let test_cases = vec![
            // Disks
            (r"C:\\Windows\\System32\\..\\..\\Temp\\conn", PathBuf::from(r"\\?\C:\Temp\conn")),
            (r"D:/../..\NUL", PathBuf::from(r"\\.\NUL")),
            (r"d:\\A\\B\\C\\D\\..\\..\\..\\..\\E\\F\\G\\..\\..\\H", PathBuf::from(r"\\?\D:\E\H")),
            (r"c:\\../Windows", PathBuf::from(r"\\?\C:\Windows")),
            (r"d:\\..\\..\\..\\..\\..", PathBuf::from(r"\\?\D:\")),

            // Verbatim Disks
            (r"\\\\?\\C:\Very\Long\Path\With\Lots\Of\Subdirectories\..\..\..\LongFile", PathBuf::from(r"\\?\C:\Very\Long\Path\With\LongFile")),
            (r"//?/d:/Trailing/Dot./.", PathBuf::from(r"\\?\d:\Trailing\Dot")),
            (r"\?\c:\Trailing\Space\\  ", PathBuf::from(r"\\?\c:\Trailing\Space\")),
            (r"\?/C:/$MFT", PathBuf::from(r"\\?\C:\$MFT")),

            // Devices
            (r"\\.\COM1", PathBuf::from(r"\\.\COM1")),
            (r"\.\PIPE\SomePipeName", PathBuf::from(r"\\.\PIPE\SomePipeName")),
            (r"/?/UNC//./PIPE/AnotherPipe", PathBuf::from(r"\\.\PIPE\AnotherPipe")),

            // Non-Standard Verbatim
            (r"\\?\Volume{12345678-1234-1234-1234-1234567890AB}\Path\To\Some\File", PathBuf::from(r"\\?\Volume{12345678-1234-1234-1234-1234567890AB}\Path\To\Some\File")),

            // UNC Verbatim
            (r"\\?\UNC\localhost\C$/Windows/System32\..\System32", PathBuf::from(r"\\?\UNC\localhost\C$\Windows\System32")),

            // Long paths
            (&long_dir_path_str, PathBuf::from(format!("\\\\?\\{temp_dir_path_str}\\{long_str}"))),
            (&long_dir_file_str, PathBuf::from(format!("\\\\?\\{temp_dir_path_str}\\{long_str}\\file.txt"))),
        ];

        for (input, expected) in test_cases {
            let result = canonical_path(input);
            assert_eq!(result, expected, "Expected canonical path for {} to be {}, but got {}", input, expected.to_string_lossy(), result.to_string_lossy());
        }
    }

    #[cfg(not(windows))]
    #[ignore]
    #[test]
    fn test_canonical_path_unix()
    {
        let cur_dir = std::env::current_dir().unwrap();

        let test_cases = vec![
            // Absolute paths
            (r"/home/.././etc/./../usr/bin", PathBuf::from(r"/usr/bin")),
            (r"/this_folder_does_not_exist/run/.././run/docker.sock", PathBuf::from(r"/this_folder_does_not_exist/run/docker.sock")),
            (r"/../../var", PathBuf::from(r"/var")),
            (r"/../../var_n/.", PathBuf::from(r"/var_n")),
            (r"///var_n//foo_n/foo_n//./././../bar_n/", PathBuf::from(r"/var_n/foo_n/bar_n/")),

            // Relative paths
            (r".", cur_dir.clone()),
            (r".//some_not_existing_folder/..", cur_dir.clone()),
            (r"./some_not_existing_folder///..//", cur_dir.join("")),
            (r"foo_n////var_n", cur_dir.join("foo_n").join("var_n")),
            (r"foo_n/../var_n/../cat_n/", cur_dir.join("cat_n")),
            (r"./foo_n/././..", cur_dir.clone()),
        ];

        for (input, expected) in test_cases {
            let result = canonical_path(input);
            assert_eq!(result, expected, "Expected canonical path for {} to be {}, but got {}", input, expected.to_string_lossy(), result.to_string_lossy());
        }
    }

    // cicd works with virtual machine, this test is slow
    #[cfg(not(all(target_arch = "aarch64", target_os = "linux")))]
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

    // cicd works with virtual machine, this test is slow
    #[cfg(not(all(target_arch = "aarch64", target_os = "linux")))]
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
        let results = fuzzy_search(&correction_candidate, paths_str, 10, &['/', '\\']);

        // Assert
        let time_spent = start_time.elapsed();
        println!("fuzzy_search took {} ms", time_spent.as_millis());
        assert!(time_spent.as_millis() < 750, "fuzzy_search took {} ms", time_spent.as_millis());

        assert_eq!(results.len(), 10, "The result should contain 10 paths");
        println!("{:?}", results);
    }
}
