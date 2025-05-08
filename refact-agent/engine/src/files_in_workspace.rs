use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Weak, Mutex as StdMutex};
use std::time::Instant;
use indexmap::IndexSet;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use ropey::Rope;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use walkdir::WalkDir;
use which::which;
use tracing::info;

use crate::files_correction::{canonical_path, CommandSimplifiedDirExt};
use crate::git::operations::git_ls_files;
use crate::global_context::GlobalContext;
use crate::integrations::running_integrations::load_integrations;
use crate::telemetry;
use crate::file_filter::{is_valid_file, SOURCE_FILE_EXTENSIONS};
use crate::ast::ast_indexer_thread::ast_indexer_enqueue_files;
use crate::privacy::{check_file_privacy, load_privacy_if_needed, PrivacySettings, FilePrivacyLevel};
use crate::files_blocklist::{
    IndexingEverywhere,
    is_blocklisted,
    reload_indexing_everywhere_if_needed,
};


// How this works
// --------------
//
// IDE Window communicates workspace folders via LSP:
//    workspace_folder1:
//       some_dir/
//          vcs_root1/
//       vcs_root2/
//    workspace_folder2:
//       dir_without_version/
//          maybe_because_its_new/
//
// We use version control (git, hg, svn) to list files, whenever we can find it.
// If we can't, just use built-in blocklist and recursive directory walk.
// When a file event arrives (such as file created, file modified) we just add the file into index, because it
// might be new (not yet in version control), but apply blocklists to avoid indexing all kinds of junk
// files.
// So blocklist is mainly useful to deal with file events.
// You can customize blocklist using:
//   ~/.config/refact/indexing.yaml
//   ~/path/to/your/project/.refact/indexing.yaml


#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Document {
    pub doc_path: PathBuf,
    pub doc_text: Option<Rope>,
}

pub async fn get_file_text_from_memory_or_disk(global_context: Arc<ARwLock<GlobalContext>>, file_path: &PathBuf) -> Result<String, String>
{
    check_file_privacy(load_privacy_if_needed(global_context.clone()).await, &file_path, &FilePrivacyLevel::AllowToSendAnywhere)?;

    if let Some(doc) = global_context.read().await.documents_state.memory_document_map.get(file_path) {
        let doc = doc.read().await;
        if doc.doc_text.is_some() {
            return Ok(doc.doc_text.as_ref().unwrap().to_string());
        }
    }
    read_file_from_disk_without_privacy_check(&file_path)
        .await.map(|x|x.to_string())
        .map_err(|e|format!("Not found in memory, not found on disk: {}", e))
}

impl Document {
    pub fn new(doc_path: &PathBuf) -> Self {
        Self { doc_path: doc_path.clone(),  doc_text: None }
    }

    #[cfg(feature="vecdb")]
    pub async fn update_text_from_disk(&mut self, gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> {
        match read_file_from_disk(load_privacy_if_needed(gcx.clone()).await, &self.doc_path).await {
            Ok(res) => {
                self.doc_text = Some(res);
                return Ok(());
            },
            Err(e) => {
                return Err(e)
            }
        }
    }

    pub async fn get_text_or_read_from_disk(&mut self, gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
        if self.doc_text.is_some() {
            return Ok(self.doc_text.as_ref().unwrap().to_string());
        }
        read_file_from_disk(load_privacy_if_needed(gcx.clone()).await, &self.doc_path).await.map(|x|x.to_string())
    }

    pub fn update_text(&mut self, text: &String) {
        self.doc_text = Some(Rope::from_str(text));
    }

    #[cfg(feature="vecdb")]
    pub fn text_as_string(&self) -> Result<String, String> {
        if let Some(r) = &self.doc_text {
            return Ok(r.to_string());
        }
        return Err(format!("no text loaded in {}", self.doc_path.display()));
    }

    pub fn does_text_look_good(&self) -> Result<(), String> {
        // Some simple tests to find if the text is suitable to parse (not generated or compressed code)
        assert!(self.doc_text.is_some());
        let r = self.doc_text.as_ref().unwrap();

        let total_chars = r.chars().count();
        let total_lines = r.lines().count();
        let avg_line_length = total_chars / total_lines;
        if avg_line_length > 150 {
            return Err("generated, avg line length > 150".to_string());
        }

        // example: hl.min.js
        let total_spaces = r.chars().filter(|x| x.is_whitespace()).count();
        let spaces_percentage = total_spaces as f32 / total_chars as f32;
        if total_lines >= 5 && spaces_percentage <= 0.05 {
            return Err(format!("generated or compressed, {:.1}% spaces < 5%", 100.0*spaces_percentage));
        }

        Ok(())
    }
}

pub struct DocumentsState {
    pub workspace_folders: Arc<StdMutex<Vec<PathBuf>>>,
    pub workspace_files: Arc<StdMutex<Vec<PathBuf>>>,
    pub workspace_vcs_roots: Arc<StdMutex<Vec<PathBuf>>>,
    pub active_file_path: Option<PathBuf>,
    pub jsonl_files: Arc<StdMutex<Vec<PathBuf>>>,
    // document_map on windows: c%3A/Users/user\Documents/file.ext
    // query on windows: C:/Users/user/Documents/file.ext
    pub memory_document_map: HashMap<PathBuf, Arc<ARwLock<Document>>>,   // if a file is open in IDE, and it's outside workspace dirs, it will be in this map and not in workspace_files
    pub cache_dirty: Arc<AMutex<f64>>,
    pub cache_correction: Arc<HashMap<String, HashSet<String>>>,  // map dir3/file.ext -> to /dir1/dir2/dir3/file.ext
    pub cache_shortened: Arc<HashSet<String>>,
    pub fs_watcher: Arc<ARwLock<RecommendedWatcher>>,
}

async fn mem_overwrite_or_create_document(
    global_context: Arc<ARwLock<GlobalContext>>,
    document: Document
) -> (Arc<ARwLock<Document>>, Arc<AMutex<f64>>, bool) {
    let mut cx = global_context.write().await;
    let doc_map = &mut cx.documents_state.memory_document_map;
    if let Some(existing_doc) = doc_map.get_mut(&document.doc_path) {
        *existing_doc.write().await = document;
        (existing_doc.clone(), cx.documents_state.cache_dirty.clone(), false)
    } else {
        let path = document.doc_path.clone();
        let darc = Arc::new(ARwLock::new(document));
        doc_map.insert(path, darc.clone());
        (darc, cx.documents_state.cache_dirty.clone(), true)
    }
}

impl DocumentsState {
    pub async fn new(
        workspace_dirs: Vec<PathBuf>,
    ) -> Self {
        let watcher = RecommendedWatcher::new(|_|{}, Default::default()).unwrap();
        Self {
            workspace_folders: Arc::new(StdMutex::new(workspace_dirs)),
            workspace_files: Arc::new(StdMutex::new(Vec::new())),
            workspace_vcs_roots: Arc::new(StdMutex::new(Vec::new())),
            active_file_path: None,
            jsonl_files: Arc::new(StdMutex::new(Vec::new())),
            memory_document_map: HashMap::new(),
            cache_dirty: Arc::new(AMutex::<f64>::new(0.0)),
            cache_correction: Arc::new(HashMap::<String, HashSet<String>>::new()),
            cache_shortened: Arc::new(HashSet::<String>::new()),
            fs_watcher: Arc::new(ARwLock::new(watcher)),
        }
    }
}

pub async fn watcher_init(
    gcx: Arc<ARwLock<GlobalContext>>
) {
    let gcx_weak = Arc::downgrade(&gcx);
    let rt = tokio::runtime::Handle::current();
    let event_callback = move |res| {
        rt.block_on(async {
            if let Ok(event) = res {
                file_watcher_event(event, gcx_weak.clone()).await;
            }
        });
    };
    let mut watcher = RecommendedWatcher::new(event_callback, Config::default()).unwrap();

    let workspace_folders: Arc<StdMutex<Vec<PathBuf>>> = gcx.read().await.documents_state.workspace_folders.clone();

    for folder in workspace_folders.lock().unwrap().iter() {
        info!("ADD WATCHER (1): {}", folder.display());
        let _ = watcher.watch(folder, RecursiveMode::Recursive);
    }

    let mut fs_watcher_on_stack = Arc::new(ARwLock::new(watcher));
    {
        let mut gcx_locked = gcx.write().await;
        std::mem::swap(&mut gcx_locked.documents_state.fs_watcher, &mut fs_watcher_on_stack);  // avoid destructor under lock
    }
}

async fn read_file_from_disk_without_privacy_check(
    path: &PathBuf,
) -> Result<Rope, String> {
    tokio::fs::read_to_string(path).await
        .map(|x|Rope::from_str(&x))
        .map_err(|e|
            format!("failed to read file {}: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e)
        )
}

pub async fn read_file_from_disk(
    privacy_settings: Arc<PrivacySettings>,
    path: &PathBuf,
) -> Result<Rope, String> {
    check_file_privacy(privacy_settings, path, &FilePrivacyLevel::AllowToSendAnywhere)?;
    read_file_from_disk_without_privacy_check(path).await
}

async fn _run_command(cmd: &str, args: &[&str], path: &PathBuf, filter_out_status: bool) -> Option<Vec<PathBuf>> {
    info!("{} EXEC {} {}", path.display(), cmd, args.join(" "));
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .current_dir_simplified(path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout.clone())
        .ok()
        .map(|s| s.lines().map(|line| {
            let trimmed = line.trim();
            if filter_out_status && trimmed.len() > 1 {
                path.join(&trimmed[1..].trim())
            } else {
                path.join(line)
            }
        }).collect())
}

async fn ls_files_under_version_control(path: &PathBuf) -> Option<Vec<PathBuf>> {
    if path.join(".git").exists() {
        git_ls_files(path)
    } else if path.join(".hg").exists() && which("hg").is_ok() {
        // Mercurial repository
        _run_command("hg", &["status", "--added", "--modified", "--clean", "--unknown", "--no-status"], path, false).await
    } else if path.join(".svn").exists() && which("svn").is_ok() {
        // SVN repository
        let files_under_vc = _run_command("svn", &["list", "-R"], path, false).await;
        let files_changed = _run_command("svn", &["status"], path, true).await;
        Some(files_under_vc.unwrap_or_default().into_iter().chain(files_changed.unwrap_or_default().into_iter()).collect())
    } else {
        None
    }
}

pub fn _ls_files(
    indexing_everywhere: &IndexingEverywhere,
    path: &PathBuf,
    recursive: bool,
    blocklist_check: bool,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = vec![];
    let mut dirs_to_visit = vec![path.clone()];

    while let Some(dir) = dirs_to_visit.pop() {
        let ls_maybe = fs::read_dir(&dir);
        if ls_maybe.is_err() {
            info!("failed to read directory {}: {}", dir.display(), ls_maybe.unwrap_err());
            continue;
        }
        let ls: fs::ReadDir = ls_maybe.unwrap();
        let entries_maybe = ls.collect::<Result<Vec<_>, _>>();
        if entries_maybe.is_err() {
            info!("failed to read directory {}: {}", dir.display(), entries_maybe.unwrap_err());
            continue;
        }
        let mut entries = entries_maybe.unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let indexing_settings = indexing_everywhere.indexing_for_path(&path);
            if recursive && path.is_dir() {
                if !blocklist_check || !is_blocklisted(&indexing_settings, &path) {
                    dirs_to_visit.push(path);
                }
            } else if path.is_file() {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}

pub fn ls_files(
    indexing_everywhere: &IndexingEverywhere,
    path: &PathBuf,
    recursive: bool,
) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Err(format!("path '{}' is not a directory", path.display()));
    }

    let indexing_settings = indexing_everywhere.indexing_for_path(path);
    let mut paths = _ls_files(indexing_everywhere, path, recursive, true).unwrap();
    if recursive {
        for additional_indexing_dir in indexing_settings.additional_indexing_dirs.iter() {
            paths.extend(_ls_files(indexing_everywhere, &PathBuf::from(additional_indexing_dir), recursive, false).unwrap());
        }
    }

    Ok(paths)
}

pub async fn detect_vcs_for_a_file_path(file_path: &Path) -> Option<(PathBuf, &'static str)> {
    let mut dir = file_path.to_path_buf();
    if dir.is_file() {
        dir.pop();
    }
    loop {
        if let Some(vcs_type) = get_vcs_type(&dir) {
            return Some((dir, vcs_type));
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn get_vcs_type(path: &Path) -> Option<&'static str> {
    if path.join(".git").is_dir() {
        Some("git")
    } else if path.join(".svn").is_dir() {
        Some("svn")
    } else if path.join(".hg").is_dir() {
        Some("hg")
    } else {
        None
    }
}

// Slow version of version control detection:
// async fn is_git_repo(directory: &PathBuf) -> bool {
//     Command::new("git")
//         .arg("rev-parse")
//         .arg("--is-inside-work-tree")
//         .current_dir(directory)
//         .output()
//         .await
//         .map(|output| output.status.success())
//         .unwrap_or(false)
// }
// async fn is_svn_repo(directory: &PathBuf) -> bool {
//     Command::new("svn")
//         .arg("info")
//         .current_dir(directory)
//         .output()
//         .await
//         .map(|output| output.status.success())
//         .unwrap_or(false)
// }
// async fn is_hg_repo(directory: &PathBuf) -> bool {
//     Command::new("hg")
//         .arg("root")
//         .current_dir(directory)
//         .output()
//         .await
//         .map(|output| output.status.success())
//         .unwrap_or(false)
// }

async fn _ls_files_under_version_control_recursive(
    all_files: &mut Vec<PathBuf>,
    vcs_folders: &mut Vec<PathBuf>,
    avoid_dups: &mut HashSet<PathBuf>,
    indexing_everywhere: &mut IndexingEverywhere,
    path: PathBuf,
    allow_files_in_hidden_folders: bool,
    ignore_size_thresholds: bool,
    check_blocklist: bool,
) {
    let mut candidates: Vec<PathBuf> = vec![crate::files_correction::canonical_path(&path.to_string_lossy().to_string())];
    let mut rejected_reasons: HashMap<String, usize> = HashMap::new();
    let mut blocklisted_dirs_cnt: usize = 0;
    while !candidates.is_empty() {
        let checkme = candidates.pop().unwrap();
        if checkme.is_file() {
            let maybe_valid = is_valid_file(
                &checkme, allow_files_in_hidden_folders, ignore_size_thresholds);
            match maybe_valid {
                Ok(_) => {
                    all_files.push(checkme.clone());
                }
                Err(e) => {
                    rejected_reasons.entry(e.to_string()).and_modify(|x| *x += 1).or_insert(1);
                    continue;
                }
            }
        }
        if checkme.is_dir() {
            if avoid_dups.contains(&checkme) {
                continue;
            }
            avoid_dups.insert(checkme.clone());
            if get_vcs_type(&checkme).is_some() {
                vcs_folders.push(checkme.clone());
            }
            if let Some(v) = ls_files_under_version_control(&checkme).await {
                // Has version control
                let indexing_yaml_path = checkme.join(".refact").join("indexing.yaml");
                if indexing_yaml_path.exists() {
                    match crate::files_blocklist::load_indexing_yaml(&indexing_yaml_path, Some(&checkme)).await {
                        Ok(indexing_settings) => {
                            for d in indexing_settings.additional_indexing_dirs.iter() {
                                let cp = crate::files_correction::canonical_path(d.as_str());
                                candidates.push(cp);
                            }
                            indexing_everywhere.vcs_indexing_settings_map.insert(checkme.to_string_lossy().to_string(), indexing_settings);
                        }
                        Err(e) => {
                            tracing::error!("failed to load indexing.yaml in {}: {}", checkme.display(), e);
                        }
                    };
                }
                for x in v.iter() {
                    let maybe_valid = is_valid_file(
                        x, allow_files_in_hidden_folders, ignore_size_thresholds);
                    match maybe_valid {
                        Ok(_) => {
                            all_files.push(x.clone());
                        }
                        Err(e) => {
                            rejected_reasons.entry(e.to_string()).and_modify(|x| *x += 1).or_insert(1);
                        }
                    }
                }

            } else {
                // Don't have version control
                let indexing_settings = indexing_everywhere.indexing_for_path(&checkme);  // this effectively only uses global blocklist
                if check_blocklist && is_blocklisted(&indexing_settings, &checkme) {
                    blocklisted_dirs_cnt += 1;
                    continue;
                }
                let new_paths: Vec<PathBuf> = WalkDir::new(checkme.clone()).max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .map(|e| crate::files_correction::canonical_path(&e.path().to_string_lossy().to_string()))
                    .filter(|e| e != &checkme)
                    .collect();
                candidates.extend(new_paths);
            }
        }
    }
    info!("when inspecting {:?} rejected files reasons:", path);
    for (reason, count) in &rejected_reasons {
        info!("    {:>6} {}", count, reason);
    }
    if rejected_reasons.is_empty() {
        info!("    no bad files at all");
    }
    info!("also the loop bumped into {} blocklisted dirs", blocklisted_dirs_cnt);
}


pub async fn retrieve_files_in_workspace_folders(
    proj_folders: Vec<PathBuf>,
    indexing_everywhere: &mut IndexingEverywhere,
    allow_files_in_hidden_folders: bool,   // true when syncing to remote container
    ignore_size_thresholds: bool,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut all_files: Vec<PathBuf> = Vec::new();
    let mut vcs_folders: Vec<PathBuf> = Vec::new();
    let mut avoid_dups: HashSet<PathBuf> = HashSet::new();
    for proj_folder in proj_folders {
        _ls_files_under_version_control_recursive(
            &mut all_files,
            &mut vcs_folders,
            &mut avoid_dups,
            indexing_everywhere,
            proj_folder.clone(),
            allow_files_in_hidden_folders,
            ignore_size_thresholds,
            true,
        ).await;
    }
    info!("in all workspace folders, VCS roots found:");
    for vcs_folder in vcs_folders.iter() {
        info!("    {}", vcs_folder.display());
    }
    (all_files, vcs_folders)
}

pub fn is_path_to_enqueue_valid(path: &PathBuf) -> Result<(), String> {
    let extension = path.extension().unwrap_or_default();
    if !SOURCE_FILE_EXTENSIONS.contains(&extension.to_str().unwrap_or_default()) {
        return Err(format!("Unsupported file extension {:?}", extension).into());
    }
    Ok(())
}

async fn enqueue_some_docs(
    gcx: Arc<ARwLock<GlobalContext>>,
    paths: &Vec<String>,
    force: bool,
) {
    info!("detected {} modified/added/removed files", paths.len());
    for d in paths.iter().take(5) {
        info!("    {}", crate::nicer_logs::last_n_chars(&d, 30));
    }
    if paths.len() > 5 {
        info!("    ...");
    }
    let (vec_db_module, ast_service) = {
        let cx = gcx.read().await;
        (cx.vec_db.clone(), cx.ast_service.clone())
    };
    #[cfg(feature="vecdb")]
    if let Some(ref mut db) = *vec_db_module.lock().await {
        db.vectorizer_enqueue_files(&paths, force).await;
    }
    #[cfg(not(feature="vecdb"))]
    let _ = vec_db_module;
    if let Some(ast) = &ast_service {
        ast_indexer_enqueue_files(ast.clone(), paths, force).await;
    }
    let (cache_correction_arc, _) = crate::files_correction::files_cache_rebuild_as_needed(gcx.clone()).await;
    let mut moar_files: Vec<PathBuf> = Vec::new();
    for p in paths {
        if !cache_correction_arc.contains_key(p) {
            moar_files.push(PathBuf::from(p.clone()));
        }
    }
    if moar_files.len() > 0 {
        info!("this made file cache dirty");
        let dirty_arc = {
            let gcx_locked = gcx.read().await;
            gcx_locked.documents_state.workspace_files.lock().unwrap().extend(moar_files);
            gcx_locked.documents_state.cache_dirty.clone()
        };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        *dirty_arc.lock().await = now + 1.0;  // next rebuild will be one second later, to prevent rapid-fire rebuilds from file events
    }
}

pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<GlobalContext>>,
    wake_up_indexers: bool,
    vecdb_only: bool,
) -> i32 {
    let folders: Vec<PathBuf> = gcx.read().await.documents_state.workspace_folders.lock().unwrap().clone();

    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let mut indexing_everywhere = crate::files_blocklist::reload_global_indexing_only(gcx.clone()).await;
    let (all_files, vcs_folders) =  retrieve_files_in_workspace_folders(
        folders,
        &mut indexing_everywhere,
        false,
        false
    ).await;
    info!("enqueue_all_files_from_workspace_folders found {} files => workspace_files", all_files.len());
    let mut workspace_vcs_roots: Arc<StdMutex<Vec<PathBuf>>> = Arc::new(StdMutex::new(vcs_folders.clone()));

    let mut old_workspace_files = Vec::new();
    let cache_dirty = {
        let mut gcx_locked = gcx.write().await;
        {
            let mut workspace_files = gcx_locked.documents_state.workspace_files.lock().unwrap();
            std::mem::swap(&mut *workspace_files, &mut old_workspace_files);
            workspace_files.extend(all_files.clone());
        }
        {
            std::mem::swap(&mut gcx_locked.documents_state.workspace_vcs_roots, &mut workspace_vcs_roots);
        }
        gcx_locked.indexing_everywhere = Arc::new(indexing_everywhere);
        gcx_locked.documents_state.cache_dirty.clone()
    };

    *cache_dirty.lock().await = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

    let (vec_db_module, ast_service) = {
        let cx_locked = gcx.read().await;
        (cx_locked.vec_db.clone(), cx_locked.ast_service.clone())
    };

    // Both vecdb and ast support paths to non-existant files (possibly previously existing files) as a way to remove them from index

    let mut updated_or_removed: IndexSet<String> = IndexSet::new();
    updated_or_removed.extend(all_files.iter().map(|file| file.to_string_lossy().to_string()));
    updated_or_removed.extend(old_workspace_files.iter().map(|p| p.to_string_lossy().to_string()));
    let paths_nodups: Vec<String> = updated_or_removed.into_iter().collect();

    #[cfg(feature="vecdb")]
    if let Some(ref mut db) = *vec_db_module.lock().await {
        db.vectorizer_enqueue_files(&paths_nodups, wake_up_indexers).await;
    }
    #[cfg(not(feature="vecdb"))]
    let _ = vec_db_module;

    if let Some(ast) = ast_service {
        if !vecdb_only {
            ast_indexer_enqueue_files(ast.clone(), &paths_nodups, wake_up_indexers).await;
        }
    }
    all_files.len() as i32
}

pub async fn on_workspaces_init(gcx: Arc<ARwLock<GlobalContext>>) -> i32
{
    // Called from lsp and lsp_like
    // Not called from main.rs as part of initialization
    let allow_experimental = gcx.read().await.cmdline.experimental;

    watcher_init(gcx.clone()).await;
    let files_enqueued = enqueue_all_files_from_workspace_folders(gcx.clone(), false, false).await;

    let gcx_clone = gcx.clone();
    tokio::spawn(async move {
        crate::git::checkpoints::init_shadow_repos_if_needed(gcx_clone).await;
    });

    // Start or connect to mcp servers
    let _ = load_integrations(gcx.clone(), allow_experimental, &["**/mcp_*".to_string()]).await;

    files_enqueued
}

pub async fn on_did_open(
    gcx: Arc<ARwLock<GlobalContext>>,
    cpath: &PathBuf,
    text: &String,
    _language_id: &String,
) {
    let mut doc = Document::new(cpath);
    doc.update_text(text);
    info!("on_did_open {}", crate::nicer_logs::last_n_chars(&cpath.display().to_string(), 30));
    let (_doc_arc, dirty_arc, mark_dirty) = mem_overwrite_or_create_document(gcx.clone(), doc).await;
    if mark_dirty {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        *dirty_arc.lock().await = now;
    }
    gcx.write().await.documents_state.active_file_path = Some(cpath.clone());
}

pub async fn on_did_close(
    gcx: Arc<ARwLock<GlobalContext>>,
    cpath: &PathBuf,
) {
    info!("on_did_close {}", crate::nicer_logs::last_n_chars(&cpath.display().to_string(), 30));
    {
        let mut cx = gcx.write().await;
        if cx.documents_state.memory_document_map.remove(cpath).is_none() {
            tracing::error!("on_did_close: failed to remove from memory_document_map {:?}", cpath.display());
        }
    }
}

pub async fn on_did_change(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    text: &String,
) {
    let t0 = Instant::now();
    let (doc_arc, dirty_arc, mark_dirty) = {
        let mut doc = Document::new(path);
        doc.update_text(text);
        let (doc_arc, dirty_arc, set_mark_dirty) = mem_overwrite_or_create_document(gcx.clone(), doc).await;
        (doc_arc, dirty_arc, set_mark_dirty)
    };

    if mark_dirty {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        *dirty_arc.lock().await = now;
    }

    gcx.write().await.documents_state.active_file_path = Some(path.clone());

    let mut go_ahead = true;
    {
        let is_it_good = is_valid_file(path, false, false);
        if is_it_good.is_err() {
            info!("{:?} ignoring changes: {}", path, is_it_good.err().unwrap());
            go_ahead = false;
        }
    }

    let cpath = doc_arc.read().await.doc_path.clone().to_string_lossy().to_string();
    if go_ahead {
        enqueue_some_docs(gcx.clone(), &vec![cpath], false).await;
    }

    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        &path.to_string_lossy().to_string(),
        text,
    ).await;

    info!("on_did_change {}, total time {:.3}s", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30), t0.elapsed().as_secs_f32());
}

pub async fn on_did_delete(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf)
{
    info!("on_did_delete {}", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30));

    let (vec_db_module, ast_service, dirty_arc) = {
        let mut cx = gcx.write().await;
        cx.documents_state.memory_document_map.remove(path);
        (cx.vec_db.clone(), cx.ast_service.clone(), cx.documents_state.cache_dirty.clone())
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    (*dirty_arc.lock().await) = now;

    #[cfg(feature="vecdb")]
    match *vec_db_module.lock().await {
        Some(ref mut db) => match db.remove_file(path).await {
            Ok(_) => {}
            Err(err) => info!("VECDB Error removing: {}", err),
        },
        None => {}
    }
    #[cfg(not(feature="vecdb"))]
    let _ = vec_db_module;
    if let Some(ast) = &ast_service {
        let cpath = path.to_string_lossy().to_string();
        ast_indexer_enqueue_files(ast.clone(), &vec![cpath], false).await;
    }
}

pub async fn add_folder(gcx: Arc<ARwLock<GlobalContext>>, fpath: &PathBuf)
{
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().push(fpath.clone());
    }
    on_workspaces_init(gcx.clone()).await;
}

pub async fn remove_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf)
{
    let was_removed = {
        let documents_state = &mut gcx.write().await.documents_state;
        let initial_len = documents_state.workspace_folders.lock().unwrap().len();
        documents_state.workspace_folders.lock().unwrap().retain(|p| p != path);
        let final_len = documents_state.workspace_folders.lock().unwrap().len();
        initial_len > final_len
    };
    if was_removed {
        tracing::info!("Folder {} was successfully removed from workspace_folders.", path.display());
        on_workspaces_init(gcx.clone()).await;
    } else {
        tracing::error!("Folder {} was not found in workspace_folders.", path.display());
    }
}

pub async fn file_watcher_event(event: Event, gcx_weak: Weak<ARwLock<GlobalContext>>)
{
    async fn on_file_change(gcx_weak: Weak<ARwLock<GlobalContext>>, event: Event) {
        let mut docs = vec![];
        let indexing_everywhere_arc;
        if let Some(gcx) = gcx_weak.clone().upgrade() {
            indexing_everywhere_arc = reload_indexing_everywhere_if_needed(gcx.clone()).await;
        } else {
            return; // the program is shutting down
        }
        for p in &event.paths {
            let indexing_settings = indexing_everywhere_arc.indexing_for_path(p);
            if is_blocklisted(&indexing_settings, &p) {  // important to filter BEFORE canonical_path
                continue;
            }

            // If it's a removed file or a valid existing file, then we can enqueue it
            if (!p.exists() && p.extension().is_some()) || is_valid_file(p, false, false).is_ok() {
                let cpath = crate::files_correction::canonical_path(p.to_string_lossy());
                docs.push(cpath.to_string_lossy().to_string());
            }
        }
        if docs.is_empty() {
            return;
        }
        // info!("EventKind::Create/Modify/Remove {} paths", event.paths.len());
        if let Some(gcx) = gcx_weak.clone().upgrade() {
            enqueue_some_docs(gcx, &docs, false).await;
        }
    }

    async fn on_dot_git_dir_change(gcx_weak: Weak<ARwLock<GlobalContext>>, event: Event) {
        if let Some(gcx) = gcx_weak.clone().upgrade() {
            // Get the path before .git component, and check if repo associated exists
            let repo_paths = event.paths.iter()
                .filter_map(|p| {
                    p.components()
                        .position(|c| c == Component::Normal(".git".as_ref()))
                        .map(|i| {
                            let repo_p = p.components().take(i).collect::<PathBuf>();
                            canonical_path(repo_p.to_string_lossy())
                        })
                })
                .map(|p| {
                    let exists = p.join(".git").exists();
                    (p.clone(), exists)
                })
                .collect::<Vec<_>>();

            if repo_paths.is_empty() {
                return;
            }

            let workspace_vcs_roots = gcx.read().await.documents_state.workspace_vcs_roots.clone();

            let mut should_reindex = false;
            {
                let mut workspace_vcs_roots_locked = workspace_vcs_roots.lock().unwrap();
                for (repo_path, exists_in_disk) in repo_paths {
                    if exists_in_disk && !workspace_vcs_roots_locked.contains(&repo_path) {
                        tracing::info!("Found .git folder in workspace: {}", repo_path.to_string_lossy());
                        should_reindex = true;
                        workspace_vcs_roots_locked.push(repo_path);
                    } else if !exists_in_disk && workspace_vcs_roots_locked.contains(&repo_path) {
                        tracing::info!("Removed .git folder from workspace: {}", repo_path.to_string_lossy());
                        should_reindex = true;
                        workspace_vcs_roots_locked.retain(|p| p != &repo_path);
                    }
                }
            }

            if should_reindex {
                tracing::info!("Reindexing all files");
                enqueue_all_files_from_workspace_folders(gcx, false, false).await;
            }
        }
    }

    match event.kind {
        // We may receive specific event that a folder is being added/removed, but not the .git itself, this happens on Unix systems
        EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder) if event.paths.iter().any(
            |p| p.components().any(|c| c == Component::Normal(".git".as_ref()))
        ) => on_dot_git_dir_change(gcx_weak.clone(), event).await,

        // In Windows, we receive generic events (Any subtype), but we receive them about each exact folder
        EventKind::Create(CreateKind::Any) | EventKind::Modify(ModifyKind::Any) | EventKind::Remove(RemoveKind::Any)
            if event.paths.iter().any(|p| p.ends_with(".git")) =>
            on_dot_git_dir_change(gcx_weak, event).await,

        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) =>
            on_file_change(gcx_weak.clone(), event).await,

        EventKind::Other | EventKind::Any | EventKind::Access(_) => {}
    }
}
