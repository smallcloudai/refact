use std::sync::Arc;
use std::time::Instant;
use std::path::{PathBuf, Component, Path};
use serde::Deserialize;
use tokio::process::Command;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context::GlobalContext;
use crate::custom_error::MapErrToString;
use crate::files_in_workspace::{detect_vcs_for_a_file_path, CacheCorrection};
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

pub async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>) -> Arc<CacheCorrection> {
    let (cache_dirty_arc, mut cache_correction_arc) = {
        let cx = global_context.read().await;
        (
            cx.documents_state.cache_dirty.clone(),
            cx.documents_state.cache_correction.clone(),
        )
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref > 0.0 && now > *cache_dirty_ref {
        info!("rebuilding files cache...");
        // NOTE: we build cache on each add/delete file inside the workspace.
        // There should be a way to build cache once and then update it.
        let start_time = Instant::now();
        let paths_from_anywhere = paths_from_anywhere(global_context.clone()).await;
        let workspace_folders = get_project_dirs(global_context.clone()).await;
        let cache_correction = CacheCorrection::build(&paths_from_anywhere, &workspace_folders);

        info!("rebuild completed in {:.3}s, over {}", start_time.elapsed().as_secs_f64(), paths_from_anywhere.len());
        cache_correction_arc = Arc::new(cache_correction);
        {
            let mut cx = global_context.write().await;
            cx.documents_state.cache_correction = cache_correction_arc.clone();
        }
        *cache_dirty_ref = 0.0;
    }

    cache_correction_arc
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

async fn _correct_to_nearest(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    is_dir: bool,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    if let Some(fixed) = complete_path_with_project_dir(gcx.clone(), correction_candidate, is_dir).await {
        return vec![fixed.to_string_lossy().to_string()];
    }

    let cache_correction_arc = files_cache_rebuild_as_needed(gcx.clone()).await;
    // it's dangerous to use cache_correction_arc without a mutex, but should be fine as long as it's read-only
    // (another thread never writes to the map itself, it can only replace the arc with a different map)

    // NOTE: do we need top_n here?
    let correction_cache = if is_dir {
        &cache_correction_arc.directories
    } else {
        &cache_correction_arc.filenames
    };
    let matches = correction_cache.find_matches(&PathBuf::from(correction_candidate));
    if matches.is_empty() {
        info!("not found {:?} in cache_correction, is_dir={}", correction_candidate, is_dir);
    } else {
        return matches.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<String>>();
    }

    if fuzzy {
        info!("fuzzy search {:?} is_dir={}, cache_fuzzy_arc.len={}", correction_candidate, is_dir, correction_cache.len());
        return fuzzy_search(correction_candidate, correction_cache.short_paths_iter(), top_n, &['/', '\\']);
    }

    vec![]
}

pub async fn correct_to_nearest_filename(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    _correct_to_nearest(gcx, correction_candidate, false, fuzzy, top_n).await
}

pub async fn correct_to_nearest_dir_path(
    gcx: Arc<ARwLock<GlobalContext>>,
    correction_candidate: &String,
    fuzzy: bool,
    top_n: usize,
) -> Vec<String> {
    _correct_to_nearest(gcx, correction_candidate, true, fuzzy, top_n).await
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
    let cache_correction_arc = files_cache_rebuild_as_needed(gcx.clone()).await;
    _shortify_paths_from_indexed(&cache_correction_arc, paths)
}

fn _shortify_paths_from_indexed(cache_correction: &CacheCorrection, paths: &Vec<String>) -> Vec<String> {
    paths.into_iter().map(|path| {
        if let Some(shortened) = cache_correction.filenames.short_path(&PathBuf::from(path)) {
            return shortened.to_string_lossy().to_string();
        }
        if let Some(shortened) = cache_correction.directories.short_path(&PathBuf::from(path)) {
            return shortened.to_string_lossy().to_string();
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

pub async fn check_if_its_inside_a_workspace_or_config(gcx: Arc<ARwLock<GlobalContext>>, path: &Path) -> Result<(), String> {
    let workspace_folders = get_project_dirs(gcx.clone()).await;
    let config_dir = gcx.read().await.config_dir.clone();

    if workspace_folders.iter().any(|d| path.starts_with(d)) || path.starts_with(&config_dir) {
        Ok(())
    } else {
        Err(format!("Path '{path:?}' is outside of project directories:\n{workspace_folders:?}"))
    }
}

pub fn any_glob_matches_path(globs: &[String], path: &Path) -> bool {
    globs.iter().any(|glob| {
        let pattern = glob::Pattern::new(glob).unwrap();
        let mut matches = pattern.matches_path(path);
        matches |= path.to_str().map_or(false, |s: &str| s.ends_with(glob));
        matches
    })
}

pub fn serialize_path<S: serde::Serializer>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}

pub fn deserialize_path<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    Ok(PathBuf::from(String::deserialize(deserializer)?))
}

pub trait CommandSimplifiedDirExt {
    /// Set current directory, in non-Windows as-is, in Windows try to remove UNC prefix if possible,
    /// since tokio::process::Command::current_dir doesn't like it
    fn current_dir_simplified<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self;
}

impl CommandSimplifiedDirExt for Command {
    fn current_dir_simplified<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.current_dir(dunce::simplified(dir.as_ref()))
    }
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
        let cache_correction = CacheCorrection::build(&paths, &workspace_folders);

        // Assert
        let mut cache_shortened_result_vec = cache_correction.filenames.short_paths_iter().collect::<Vec<_>>();
        let mut expected_result = vec![
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo1").join("this_file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir").join("this_file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").to_string_lossy().to_string(),
        ];

        expected_result.sort();
        cache_shortened_result_vec.sort();

        assert_eq!(cache_correction.filenames.len(), 5, "The cache should contain 5 paths");
        assert_eq!(cache_shortened_result_vec, expected_result, "The result should contain the expected paths, instead it found");
    }

    #[test]
    fn test_shortify_paths_from_indexed() {
        let workspace_folders = vec![
            PathBuf::from("home").join("user").join("repo1"),
            PathBuf::from("home").join("user").join("repo1").join("nested").join("repo2"),
            PathBuf::from("home").join("user").join("repo3"),
        ];

        let indexed_paths = vec![
            PathBuf::from("home").join("user").join("repo1").join("dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo1").join("nested").join("repo2").join("dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo3").join("dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo1").join("this_file.ext"),
            PathBuf::from("home").join("user").join("repo1").join(".hidden").join("custom_dir").join("file.ext"),
            PathBuf::from("home").join("user").join("repo3").join("dir2").join("another_file.ext"),
        ];

        let paths = vec![
            PathBuf::from("home").join("user").join("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo1").join("nested").join("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo3").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo3").join("dir2").join("another_file.ext").to_string_lossy().to_string(),
            // Hidden file; should not be shortened as it's not in the cache and may be confused with custom_dir/file.ext.
            PathBuf::from("home").join("user").join("repo4").join(".hidden").join("custom_dir").join("file.ext").to_string_lossy().to_string(),
        ];

        // _shortify_paths_from_indexed
        let cache_correction = CacheCorrection::build(&indexed_paths, &workspace_folders);
        let mut result = _shortify_paths_from_indexed(&cache_correction, &paths);

        let mut expected_result = vec![
            PathBuf::from("repo1").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("nested").join("repo2").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("repo3").join("dir").join("file.ext").to_string_lossy().to_string(),
            PathBuf::from("dir2").join("another_file.ext").to_string_lossy().to_string(),
            PathBuf::from("home").join("user").join("repo4").join(".hidden").join("custom_dir").join("file.ext").to_string_lossy().to_string(),
        ];

        result.sort();
        expected_result.sort();

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
        let workspace_folders = vec![
            PathBuf::from("home").join("user").join("repo1"),
            PathBuf::from("home").join("user").join("repo2"),
            PathBuf::from("home").join("user").join("repo3"),
            PathBuf::from("home").join("user").join("repo4"),
        ];

        let mut paths = Vec::new();
        for i in 0..100000 {
            let path = workspace_folders[i % workspace_folders.len()]
                .join(format!("dir{}", i % 1000))
                .join(format!("dir{}", i / 1000))
                .join(format!("file{}.ext", i));
            paths.push(path);
        }
        let start_time = Instant::now();

        // Act
        let cache_correction = CacheCorrection::build(&paths, &workspace_folders);
        let cache_shortened_result_vec = cache_correction.filenames.short_paths_iter().collect::<Vec<_>>();

        // Assert
        let time_spent = start_time.elapsed();
        println!("make_cache took {} ms", time_spent.as_millis());
        assert!(time_spent.as_millis() < 2500, "make_cache took {} ms", time_spent.as_millis());

        assert_eq!(cache_correction.filenames.len(), paths.len(), "The cache should contain 100000 paths");
        assert_eq!(cache_shortened_result_vec.len(), paths.len(), "The cache shortened should contain 100000 paths");
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
