use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{Arc, Weak, Mutex};
use std::time::Instant;
use crate::global_context::GlobalContext;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind};
use ropey::Rope;
use tokio::runtime::Runtime;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use tracing::info;
use walkdir::WalkDir;
use which::which;

use crate::telemetry;
use crate::vecdb::file_filter::{is_this_inside_blacklisted_dir, is_valid_file, BLACKLISTED_DIRS};


#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Document {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub language_id: String,
    pub text: Option<Rope>,
    pub in_jsonl: bool,
}

pub async fn read_file_from_disk(path: &PathBuf) -> Result<Rope, String> {
    tokio::fs::read_to_string(path).await
        .map(|x|Rope::from_str(&x))
        .map_err(|e| format!("Failed to read file from disk: {}", e))
}

pub fn read_file_from_disk_block(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file from disk: {}", e))
}

impl Document {
    pub fn new(path: &PathBuf, language_id: Option<String>) -> Self {
        let language_id = language_id.unwrap_or("unknown".to_string());
        Self { path: path.clone(), language_id, text: None, in_jsonl: false }
    }
    pub async fn update_text_from_disk(&mut self) {
        if let Ok(res) = read_file_from_disk(&self.path).await {
            self.text = Some(res);
        }
    }
    pub async fn get_text_or_read_from_disk(&mut self) -> Result<String, String> {
        if let Some(text) = self.text.clone() {
            return Ok(text.to_string());
        }
        read_file_from_disk(&self.path).await.map(|x|x.to_string())
    }
    pub fn update_text(&mut self, text: &String) {
        self.text = Some(Rope::from_str(text));
    }
}

pub struct DocumentsState {
    pub workspace_folders: Arc<Mutex<Vec<PathBuf>>>,
    pub workspace_files: Arc<Mutex<Vec<PathBuf>>>,
    // document_map on windows: c%3A/Users/user\Documents/file.ext
    // query on windows: C:/Users/user/Documents/file.ext
    pub document_map: HashMap<PathBuf, Arc<ARwLock<Document>>>,   // if a file is open in IDE, and it's outside workspace dirs, it will be in this map and not in workspace_files
    pub cache_dirty: Arc<AMutex<bool>>,
    pub cache_correction: Arc<HashMap<String, String>>,  // map dir3/file.ext -> to /dir1/dir2/dir3/file.ext
    pub cache_fuzzy: Arc<Vec<String>>,                   // slow linear search
    pub fs_watcher: Arc<ARwLock<RecommendedWatcher>>,
}

async fn add_paths_to_document_map_if_not_present(
    global_context: Arc<ARwLock<GlobalContext>>,
    paths: &Vec<PathBuf>,
    read_text: bool,
) {
    let mut cx = global_context.write().await;
    let doc_map = &mut cx.documents_state.document_map;
    for path in paths {
        if !doc_map.contains_key(path) {
            let mut doc_new = Document::new(path, None);
            if read_text {
                doc_new.update_text_from_disk().await;
            }
            doc_map.insert(path.clone(), Arc::new(ARwLock::new(doc_new)));
        }
    }
}

async fn overwrite_or_create_document(global_context: Arc<ARwLock<GlobalContext>>, document: Document) {
    let mut cx = global_context.write().await;
    let doc_map = &mut cx.documents_state.document_map;
    if let Some(existing_doc) = doc_map.get_mut(&document.path) {
        *existing_doc.write().await = document;
    } else {
        doc_map.insert(document.path.clone(), Arc::new(ARwLock::new(document)));
    }
}

impl DocumentsState {
    pub async fn new(
        workspace_dirs: Vec<PathBuf>
    ) -> Self {
        let watcher = RecommendedWatcher::new(|_|{}, Default::default()).unwrap();
        Self {
            workspace_folders: Arc::new(Mutex::new(workspace_dirs)),
            workspace_files: Arc::new(Mutex::new(Vec::new())),
            document_map: HashMap::new(),
            cache_dirty: Arc::new(AMutex::<bool>::new(false)),
            cache_correction: Arc::new(HashMap::<String, String>::new()),
            cache_fuzzy: Arc::new(Vec::<String>::new()),
            fs_watcher: Arc::new(ARwLock::new(watcher)),
        }

    }

    pub fn init_watcher(&mut self, gcx: Arc<ARwLock<GlobalContext>>) {
        let gcx_cloned = Arc::downgrade(&gcx.clone());
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(event) = res {
                        file_watcher_thread(event, gcx_cloned.clone()).await;
                    }
                })
            },
            Config::default(),
        ).unwrap();
        for folder in self.workspace_folders.lock().unwrap().iter() {
            watcher.watch(folder, RecursiveMode::Recursive).unwrap();
        }
        self.fs_watcher = Arc::new(ARwLock::new(watcher));
    }
}

pub async fn get_file_text_from_memory_or_disk(global_context: Arc<ARwLock<GlobalContext>>, file_path: &PathBuf) -> Result<String, String> {
    if let Some(doc) = global_context.read().await.documents_state.document_map.get(file_path) {
        let doc = doc.read().await;
        if doc.text.is_some() {
            return Ok(doc.text.clone().unwrap().to_string());
        }
    }
    read_file_from_disk(&file_path).await.map(|x|x.to_string())
}

async fn _run_command(cmd: &str, args: &[&str], path: &PathBuf) -> Option<Vec<PathBuf>> {
    info!("{} EXEC {} {}", path.display(), cmd, args.join(" "));
    let output = async_process::Command::new(cmd)
        .args(args)
        .current_dir(path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout.clone())
        .ok()
        .map(|s| s.lines().map(|line| path.join(line)).collect())
}

async fn ls_files_under_version_control(path: &PathBuf) -> Option<Vec<PathBuf>> {
    if path.join(".git").exists() && which("git").is_ok() {
        // Git repository
        _run_command("git", &["ls-files"], path).await
    } else if path.join(".hg").exists() && which("hg").is_ok() {
        // Mercurial repository
        _run_command("hg", &["status", "-c"], path).await
    } else if path.join(".svn").exists() && which("svn").is_ok() {
        // SVN repository
        _run_command("svn", &["list", "-R"], path).await
    } else {
        None
    }
}

async fn ls_files_under_version_control_recursive(path: PathBuf) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![];
    let mut candidates: Vec<PathBuf> = vec![path];
    let mut rejected_reasons: HashMap<String, usize> = HashMap::new();
    let mut blacklisted_dirs_cnt: usize = 0;
    while !candidates.is_empty() {
        let local_path = candidates.pop().unwrap();
        if local_path.is_file() {
            let maybe_valid = is_valid_file(&local_path);
            match maybe_valid {
                Ok(_) => {
                    paths.push(local_path.clone());
                }
                Err(e) => {
                    rejected_reasons.entry(e.to_string()).and_modify(|x| *x += 1).or_insert(1);
                    continue;
                }
            }
        }
        if local_path.is_dir() {
            if BLACKLISTED_DIRS.contains(&local_path.file_name().unwrap().to_str().unwrap()) {
                blacklisted_dirs_cnt += 1;
                continue;
            }
            let maybe_files = ls_files_under_version_control(&local_path).await;
            if let Some(files) = maybe_files {
                paths.extend(files);
            } else {
                let local_paths: Vec<PathBuf> = WalkDir::new(local_path.clone()).max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .map(|e| e.path().to_path_buf())
                    .filter(|e| e != &local_path)
                    .collect();
                candidates.extend(local_paths);
            }
        }
    }
    info!("rejected files reasons:");
    for (reason, count) in &rejected_reasons {
        info!("    {:>6} {}", count, reason);
    }
    if rejected_reasons.is_empty() {
        info!("    no bad files at all");
    }
    info!("also the loop bumped into {} blacklisted dirs", blacklisted_dirs_cnt);
    paths
}

async fn retrieve_files_by_proj_folders(proj_folders: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    for proj_folder in proj_folders {
        let files = ls_files_under_version_control_recursive(proj_folder.clone()).await;
        all_files.extend(files);
    }
    all_files
}

async fn enqueue_docs(
    gcx: Arc<ARwLock<GlobalContext>>,
    docs: &Vec<Arc<ARwLock<Document>>>,
) {
    let mut documents = vec![];
    for d in docs {
        documents.push(d.read().await.clone())
    }
    let (vec_db_module, ast_module) = {
        let cx = gcx.write().await;
        (cx.vec_db.clone(), cx.ast_module.clone())
    };

    match *vec_db_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&documents, false).await,
        None => {}
    }
    match &ast_module {
        Some(ast) => ast.read().await.ast_indexer_enqueue_files(&documents, true).await,
        None => {}
    };
}

pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> i32 {
    let folders: Vec<PathBuf> = gcx.read().await.documents_state.workspace_folders.lock().unwrap().clone();

    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let paths = retrieve_files_by_proj_folders(folders).await;
    info!("enqueue_all_files_from_workspace_folders found {} files => workspace_files", paths.len());

    add_paths_to_document_map_if_not_present(gcx.clone(), &paths, true).await;
    let docs = gcx.read().await.documents_state.document_map.iter().map(|(_, v)|v.clone()).collect::<Vec<_>>();
    let mut documents: Vec<Document> = vec![];
    for d in docs {
        documents.push(d.read().await.clone());
    }

    let (vec_db_module, ast_module) = {
        let cx = gcx.write().await;
        *cx.documents_state.cache_dirty.lock().await = true;
        let workspace_files = &mut cx.documents_state.workspace_files.lock().unwrap();
        workspace_files.clear();
        workspace_files.extend(paths);
        (cx.vec_db.clone(), cx.ast_module.clone())
    };

    match *vec_db_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&documents, true).await,
        None => {}
    }
    match &ast_module {
        Some(ast) => ast.read().await.ast_indexer_enqueue_files(&documents, false).await,
        None => {}
    };

    documents.len() as i32
}

pub async fn on_workspaces_init(gcx: Arc<ARwLock<GlobalContext>>) -> i32 {
    enqueue_all_files_from_workspace_folders(gcx.clone()).await
}

pub async fn on_did_open(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    text: &String,
    language_id: &String,
) {
    let mut doc = Document::new(path, Some(language_id.clone()));
    doc.update_text(text);
    info!("on_did_open {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30));
    overwrite_or_create_document(gcx.clone(), doc).await;
    *gcx.write().await.documents_state.cache_dirty.lock().await = true;
}

pub async fn on_did_change(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    text: &String,
) {
    let t0 = Instant::now();
    let mut mark_dirty: bool = false;

    let doc_mb = {
        if let Some(ex_doc) = gcx.write().await.documents_state.document_map.get(path) {
            ex_doc.write().await.update_text(text);
            Some(ex_doc.clone())
        } else {
            info!("WARNING: file {} reported changed, but this binary has no record of this file.", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30));
            let mut doc = Document::new(path, None);
            doc.update_text(text);
            overwrite_or_create_document(gcx.clone(), doc).await;
            mark_dirty = true;
            gcx.read().await.documents_state.document_map.get(path).map(|x|x.clone())
        }
    };
    let doc = match doc_mb {
        Some(doc) => doc.read().await.clone(),
        None => return
    };

    *gcx.write().await.documents_state.cache_dirty.lock().await = mark_dirty;

    if is_valid_file(path).is_ok() {
        let (vec_db_module, ast_module) = {
            let cx = gcx.write().await;
            (cx.vec_db.clone(), cx.ast_module.clone())
        };

        match *vec_db_module.lock().await {
            Some(ref mut db) => db.vectorizer_enqueue_files(&vec![doc.clone()], false).await,
            None => {}
        }

        match &ast_module {
            Some(ast) => ast.read().await.ast_indexer_enqueue_files(&vec![doc.clone()], false).await,
            None => {}
        };
    }

    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        &path.to_string_lossy().to_string(),
        text,
    ).await;

    info!("on_did_change {}, total time {:.3}s", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30), t0.elapsed().as_secs_f32());
}

pub async fn on_did_delete(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf) {
    info!("on_did_delete {}", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30));

    gcx.write().await.documents_state.document_map.remove(path);
    *gcx.write().await.documents_state.cache_dirty.lock().await = true;

    let (vec_db_module, ast_module) = {
        let cx = gcx.write().await;
        (cx.vec_db.clone(), cx.ast_module.clone())
    };

    match *vec_db_module.lock().await {
        Some(ref mut db) => db.remove_file(path).await,
        None => {}
    }

    match &ast_module {
        Some(ast) => ast.write().await.remove_file(path).await,
        None => {}
    };}

pub async fn add_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf) {
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().push(path.clone());
        let _ = documents_state.fs_watcher.write().await.watch(&path.clone(), RecursiveMode::Recursive);
    }
    let paths = retrieve_files_by_proj_folders(vec![path.clone()]).await;
    add_paths_to_document_map_if_not_present(gcx.clone(), &paths, false).await;

    let mut docs = vec![];
    for d in gcx.read().await.documents_state.document_map.values() {
        docs.push(d.read().await.clone());
    }

    let (vec_db_module, ast_module) = {
        let cx = gcx.write().await;
        (cx.vec_db.clone(), cx.ast_module.clone())
    };

    match *vec_db_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, false).await,
        None => {}
    }
    match &ast_module {
        Some(ast) => ast.read().await.ast_indexer_enqueue_files(&docs, false).await,
        None => {}
    };
}

pub async fn remove_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf) {
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().retain(|p| p != path);
        let _ = documents_state.fs_watcher.write().await.unwatch(&path.clone());
    }

    let ast_module = gcx.write().await.ast_module.clone();

    if let Some(ast) = &ast_module {
        ast.read().await.ast_reset_index().await;
    }
    enqueue_all_files_from_workspace_folders(gcx.clone()).await;
}

pub async fn file_watcher_thread(event: Event, gcx: Weak<ARwLock<GlobalContext>>) {
    async fn on_create_modify(gcx: Weak<ARwLock<GlobalContext>>, event: Event) {
        let mut docs = vec![];
        for path in &event.paths {
            if is_this_inside_blacklisted_dir(path) {
                continue;
            }
            if is_valid_file(path).is_ok() {
                let mut doc = Document::new(path, None);
                doc.update_text_from_disk().await;
                docs.push(doc);
            }
        }
        if docs.is_empty() {
            return;
        }
        info!("EventKind::Create/Modify {:?}", event.paths);
        if let Some(gcx) = gcx.upgrade() {
            for doc in &docs {
                overwrite_or_create_document(gcx.clone(), doc.clone()).await;
            }
            info!("=> enqueue {} of them", docs.len());
            if event.kind == EventKind::Create(CreateKind::File) {
                gcx.clone().write().await.documents_state.workspace_files.lock().unwrap().extend(docs.iter().map(|x|x.path.clone()));
            }
            let docs_paths: HashSet<_> = docs.iter().map(|d| d.path.clone()).collect();
            let docs_gcx: Vec<_> = gcx.clone().write().await.documents_state.document_map.iter()
                .filter_map(|(k, v)| {
                    if docs_paths.contains(k) {
                        Some(v)
                    } else {
                        None
                    }
                }).cloned().collect();
            enqueue_docs(gcx, &docs_gcx).await;
        }
    }

    async fn on_remove(gcx: Weak<ARwLock<GlobalContext>>, event: Event) {
        let mut never_mind = true;
        for p in &event.paths {
            never_mind &= is_this_inside_blacklisted_dir(&p);
        }
        if !never_mind {
            info!("EventKind::Remove {:?}", event.paths);
            info!("Likely a useful file was removed, rebuild index");
            if let Some(gcx) = gcx.upgrade() {
                enqueue_all_files_from_workspace_folders(gcx).await;
            }
        }
    }

    match event.kind {
        EventKind::Any => {},
        EventKind::Access(_) => {},
        EventKind::Create(CreateKind::File) | EventKind::Modify(ModifyKind::Data(DataChange::Content)) => on_create_modify(gcx.clone(), event).await,
        EventKind::Remove(RemoveKind::File) => on_remove(gcx.clone(), event).await,
        EventKind::Other => {}
        _ => {}
    }
}
