use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Instant;
use crate::global_context::GlobalContext;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind};
use ropey::Rope;
use tokio::fs::read_to_string;
use tokio::runtime::Runtime;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};

use tracing::info;
use url::Url;
use walkdir::WalkDir;
use which::which;

use crate::global_context;
use crate::telemetry;
use crate::vecdb::file_filter::is_valid_file;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Document {
    #[allow(dead_code)]
    pub language_id: String,
    pub text: Rope,
}

impl Document {
    pub fn new(language_id: String, text: Rope) -> Self {
        Self { language_id, text }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct DocumentInfo {
    pub uri: Url,
    pub document: Option<Document>,
}

impl DocumentInfo {
    pub fn new(uri: Url) -> Self {
        Self { uri, document: None }
    }
}

impl PartialEq<Self> for DocumentInfo {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
    }
}

impl Hash for DocumentInfo {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.uri.hash(state);
    }
}

impl DocumentInfo {
    pub fn from_pathbuf(path: &PathBuf) -> Result<Self, String> {
        match pathbuf_to_url(path) {
            Ok(uri) => Ok(Self { uri, document: None }),
            Err(_) => Err("Failed to convert path to URL".to_owned())
        }
    }

    pub fn from_pathbuf_and_text(path: &PathBuf, text: &String) -> Result<Self, String> {
        match pathbuf_to_url(path) {
            Ok(uri) => Ok(Self {
                uri,
                document: Some(Document {
                    language_id: "unknown".to_string(),
                    text: Rope::from_str(&text),
                }),
            }),
            Err(_) => Err("Failed to convert path to URL".to_owned())
        }
    }

    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(self.uri.path())
    }

    pub async fn read_file(&self) -> io::Result<String> {
        match &self.document {
            Some(doc) => Ok(doc.text.to_string()),
            None => {
                read_to_string(self.get_path()).await
            }
        }
    }

    pub fn read_file_blocked(&self) -> io::Result<String> {
        match &self.document {
            Some(doc) => Ok(doc.text.to_string()),
            None => {
                std::fs::read_to_string(self.get_path())
            }
        }
    }
}


pub struct DocumentsState {
    pub workspace_folders: Arc<StdMutex<Vec<PathBuf>>>,
    pub workspace_files: Arc<StdMutex<Vec<Url>>>,
    pub document_map: Arc<ARwLock<HashMap<Url, Document>>>,   // if a file is open in IDE and it's outside workspace dirs, it will be in this map and not in workspace_files
    pub cache_dirty: Arc<AMutex<bool>>,
    pub cache_correction: Arc<HashMap<String, String>>,  // map dir3/file.ext -> to /dir1/dir2/dir3/file.ext
    pub cache_fuzzy: Arc<Vec<String>>,                   // slow linear search
    pub fs_watcher: Option<RecommendedWatcher>,
}


impl DocumentsState {
    pub fn empty(workspace_dirs: Vec<PathBuf>) -> Self {
        Self {
            workspace_folders: Arc::new(StdMutex::new(workspace_dirs)),
            workspace_files: Arc::new(StdMutex::new(vec![])),
            document_map: Arc::new(ARwLock::new(HashMap::new())),
            cache_dirty: Arc::new(AMutex::<bool>::new(false)),
            cache_correction: Arc::new(HashMap::<String, String>::new()),
            cache_fuzzy: Arc::new(Vec::<String>::new()),
            fs_watcher: None,
        }
    }

    pub fn init_watcher(&mut self, gcx: Arc<ARwLock<GlobalContext>>) {
        let watcher = RecommendedWatcher::new(
            move |res| {
                let rt  = Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(event) = res {
                        file_watcher_thread(event, gcx.clone()).await;
                    }
                })
            },
            Config::default(),
        ).unwrap();
        self.fs_watcher = Some(watcher);
    }

    pub fn finish(&mut self) {
        self.fs_watcher = None;
    }
}


pub async fn get_file_text_from_memory_or_disk(global_context: Arc<ARwLock<GlobalContext>>, file_path: &String) -> Result<String, String> {
    // if you write pathbuf_to_url(&PathBuf::from(file_path)) without unwrapping it gives: future cannot be sent between threads safe
    let url_mb = pathbuf_to_url(&PathBuf::from(file_path)).map(|x| Some(x)).unwrap_or(None);
    if let Some(url) = url_mb {
        let document_mb = global_context.read().await.documents_state.document_map.read().await.get(&url).cloned();
        if document_mb.is_some() {
            return Ok(document_mb.unwrap().text.to_string());
        }
    }

    let doc_info = match DocumentInfo::from_pathbuf(&PathBuf::from(file_path)) {
        Ok(doc) => doc.read_file().await,
        Err(_) => {
            return Err(format!("cannot parse filepath: {file_path}"));
        }
    };
    doc_info.map_err(|e| e.to_string())
}

pub fn pathbuf_to_url(path: &PathBuf) -> Result<Url, Box<dyn std::error::Error>> {
    let absolute_path = if path.is_absolute() {
        path.clone()
    } else {
        let path = std::env::current_dir()?.join(path);
        path
    };
    let url = Url::from_file_path(absolute_path).map_err(|_| "Failed to convert path to URL")?;
    Ok(url)
}

async fn _run_command(cmd: &str, args: &[&str], path: &PathBuf) -> Option<Vec<PathBuf>> {
    info!("EXEC {} {}", cmd, args.join(" "));
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

async fn _ls_files_under_version_control(path: &PathBuf) -> Option<Vec<PathBuf>> {
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
        Some(glob_folder(path).await)
    }
}

pub async fn _retrieve_files_by_proj_folders(proj_folders: Vec<PathBuf>) -> Vec<DocumentInfo> {
    let mut all_files: Vec<DocumentInfo> = Vec::new();
    for proj_folder in proj_folders {
        let maybe_files = _ls_files_under_version_control(&proj_folder).await;
        if let Some(files) = maybe_files {
            all_files.extend(files.iter().filter_map(|x| DocumentInfo::from_pathbuf(x).ok()).collect::<Vec<_>>());
        } else {
            let files: Vec<DocumentInfo> = WalkDir::new(proj_folder)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| !e.path().is_dir())
                .filter(|e| is_valid_file(&e.path().to_path_buf()))
                .filter_map(|e| DocumentInfo::from_pathbuf(&e.path().to_path_buf()).ok())
                .collect::<Vec<DocumentInfo>>();
            all_files.extend(files);
        }
    }
    all_files
}

pub(crate) async fn glob_folder(path: &PathBuf) -> Vec<PathBuf> {
    let mut docs = Vec::new();
    let mut files = glob::glob(path.join("**").to_str().unwrap()).unwrap();
    while let Some(file_res) = files.next() {
        if let Ok(file) = file_res {
            docs.push(file);
        }
    }
    docs
}

async fn enqueue_files(
    gcx: Arc<ARwLock<GlobalContext>>,
    docs: Vec<DocumentInfo>,
) {
    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&docs, true).await,
        None => {}
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, false).await,
        None => {}
    };
}

pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    let folders: Vec<PathBuf> = {
        let cx_locked = gcx.read().await;
        let x = cx_locked.documents_state.workspace_folders.lock().unwrap().clone();
        x
    };
    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let docs = _retrieve_files_by_proj_folders(folders).await;
    info!("enqueue_all_files_from_workspace_folders found {} files", docs.len());
    let tmp = docs.iter().map(|x| x.uri.clone()).collect::<Vec<_>>();

    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.write().await;
        {
            *cx_locked.documents_state.cache_dirty.lock().await = true;
        }
        let workspace_files: &mut Vec<Url> = &mut cx_locked.documents_state.workspace_files.lock().unwrap();
        workspace_files.clear();
        workspace_files.extend(tmp);
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&docs, false).await,
        None => {
            info!("ast_module is None");
        }
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, true).await,
        None => {}
    };
    docs.len() as i32
}

pub async fn on_workspaces_init(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    // TODO: this will not work when files change. Need a real file watcher.
    enqueue_all_files_from_workspace_folders(gcx.clone()).await
}

pub async fn on_did_open(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    file_url: &Url,
    text: &String,
    language_id: &String,
) {
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.documents_state.document_map;
    let mut document_map_locked = document_map.write().await;
    let doc = Document::new(language_id.clone(), Rope::from_str(&text));
    let doc_info = DocumentInfo { uri: file_url.clone(), document: Some(doc.clone()) };
    document_map_locked.insert(file_url.clone(), doc);
    let path_str = format!("{:?}", doc_info.get_path());
    let last_30_chars: String = crate::nicer_logs::last_n_chars(&path_str, 30);
    info!("opened {}", last_30_chars);
}


pub async fn on_did_change(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    file_url: &Url,
    text: &String,
) {
    let t0 = Instant::now();
    let doc_info = {
        let gcx_locked = gcx.read().await;
        let document_map = &gcx_locked.documents_state.document_map;
        let mut document_map_locked = document_map.write().await;
        let doc = document_map_locked.entry(file_url.clone())
            .or_insert(Document::new("unknown".to_owned(), Rope::new()));
        doc.text = Rope::from_str(&text);
        DocumentInfo { uri: file_url.clone(), document: Some(doc.clone()) }
    };
    if is_valid_file(&doc_info.get_path()) {
        {
            let vecdb_bind = gcx.read().await.vec_db.clone();
            match *vecdb_bind.lock().await {
                Some(ref mut db) => db.vectorizer_enqueue_files(&vec![doc_info.clone()], false).await,
                None => {}
            };
        }
        {
            let ast_bind = gcx.read().await.ast_module.clone();
            match *ast_bind.lock().await {
                Some(ref mut ast) => ast.ast_indexer_enqueue_files(&vec![doc_info.clone()], false).await,
                None => {}
            };
        }
    }

    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        &doc_info.uri.to_string(),
        text,
    ).await;
    let last_30_chars: String = crate::nicer_logs::last_n_chars(&doc_info.get_path().display().to_string(), 30);
    info!("changed {}, total time {:.3}s", last_30_chars, t0.elapsed().as_secs_f32());
}

pub async fn on_did_delete(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    file_url: &Url,
) {
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.documents_state.document_map;
    let mut document_map_locked = document_map.write().await;
    document_map_locked.remove(file_url);
    {
        let file_path = PathBuf::from(file_url.path());
        let path_str = format!("{:?}", file_path);
        let last_30_chars: String = crate::nicer_logs::last_n_chars(&path_str, 30);
        info!("deleted {}", last_30_chars);
    }

    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };

    {
        match *vecdb_module.lock().await {
            Some(ref mut db) => {
                let file_path = PathBuf::from(file_url.path());
                db.remove_file(&file_path).await
            }
            None => {}
        };
    }
    {
        match *ast_module.lock().await {
            Some(ref mut ast) => {
                let doc = DocumentInfo {
                    uri: file_url.clone(),
                    document: None,
                };
                ast.remove_file(&doc).await
            }
            None => {}
        };
    }
}

pub async fn add_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf) {
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().push(path.clone());
        if let Some(watcher) = documents_state.fs_watcher.as_mut() {
            let _ = watcher.watch(&path.clone(), RecursiveMode::Recursive);
        }
    }
    let docs = _retrieve_files_by_proj_folders(vec![path.clone()]).await;

    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&docs, false).await,
        None => {}
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, false).await,
        None => {}
    };
}

pub async fn remove_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf) {
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().retain(|p| p != path);
        if let Some(watcher) = documents_state.fs_watcher.as_mut() {
            let _ = watcher.unwatch(&path.clone());
        }
        // let _ = documents_state.fs_watcher.unwatch(&path.clone());
    }
    let (ast_module, _vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_reset_index().await,
        None => {}
    };

    enqueue_all_files_from_workspace_folders(gcx.clone()).await;
}

pub async fn file_watcher_thread(event: Event, gcx: Arc<ARwLock<GlobalContext>>) {
    match event.kind {
        EventKind::Any => {}
        EventKind::Access(_) => {}
        EventKind::Create(CreateKind::File) | EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
            let docs = event.paths.iter().map(|path| DocumentInfo::new(pathbuf_to_url(path).unwrap())).collect();
            enqueue_files(gcx.clone(), docs).await;
        }
        EventKind::Remove(RemoveKind::File) => {
            enqueue_all_files_from_workspace_folders(gcx.clone()).await;
        }
        EventKind::Other => {}
        _ => {}
    }
}
