use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::{Component, PathBuf};
use std::sync::{Arc, Weak, Mutex as StdMutex};
use std::time::Instant;
use crate::global_context::GlobalContext;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind};
use ropey::Rope;
use tokio::runtime::Runtime;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use strsim::normalized_damerau_levenshtein;

use tracing::{info, warn};
use walkdir::WalkDir;
use which::which;

use crate::telemetry;
use crate::vecdb::file_filter::{is_this_inside_blacklisted_dir, is_valid_file, BLACKLISTED_DIRS};


#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Document {
    pub path: PathBuf,
    // #[allow(dead_code)]
    // pub language_id: String,
    pub text: Option<Rope>,
}

pub async fn files_cache_rebuild_as_needed(global_context: Arc<ARwLock<GlobalContext>>) -> (Arc<HashMap<String, String>>, Arc<Vec<String>>)
{
    let cache_dirty_arc: Arc<AMutex<bool>>;
    let mut cache_correction_arc: Arc<HashMap<String, String>>;
    let mut cache_fuzzy_arc: Arc<Vec<String>>;
    {
        let gcx_locked = global_context.read().await;
        cache_dirty_arc = gcx_locked.documents_state.cache_dirty.clone();
        cache_correction_arc = gcx_locked.documents_state.cache_correction.clone();
        cache_fuzzy_arc = gcx_locked.documents_state.cache_fuzzy.clone();
    }
    let mut cache_dirty_ref = cache_dirty_arc.lock().await;
    if *cache_dirty_ref {
        // Rebuild, cache_dirty_arc stays locked.
        // Any other thread will wait at this if until the rebuild is complete.
        // Sources:
        // - documents_state.document_map
        // - cx_locked.documents_state.workspace_files
        // - global_context.read().await.cmdline.files_jsonl_path
        info!("rebuilding files cache...");
        let file_paths_from_memory = global_context.read().await.documents_state.memory_document_map.keys().map(|x|x.clone()).collect::<Vec<_>>();
        let paths_from_workspace: Vec<PathBuf> = global_context.read().await.documents_state.workspace_files.lock().unwrap().clone();
        let paths_from_jsonl: Vec<PathBuf> = global_context.read().await.documents_state.jsonl_files.lock().unwrap().clone();

        let mut cache_correction = HashMap::<String, String>::new();
        let mut cache_fuzzy_set = HashSet::<String>::new();
        let mut cnt = 0;

        let paths_from_anywhere = file_paths_from_memory.into_iter().chain(paths_from_workspace.into_iter().chain(paths_from_jsonl.into_iter()));
        for path in paths_from_anywhere {
            let path_str = path.to_str().unwrap_or_default().to_string();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            cache_fuzzy_set.insert(file_name);
            cnt += 1;

            cache_correction.insert(path_str.clone(), path_str.clone());
            // chop off directory names one by one
            let mut index = 0;
            while let Some(slashpos) = path_str[index .. ].find(|c| c == '/' || c == '\\') {
                let absolute_slashpos = index + slashpos;
                index = absolute_slashpos + 1;
                let slashpos_to_end = &path_str[index .. ];
                if !slashpos_to_end.is_empty() {
                    cache_correction.insert(slashpos_to_end.to_string(), path_str.clone());
                }
            }
        }
        let cache_fuzzy: Vec<String> = cache_fuzzy_set.into_iter().collect();
        info!("rebuild over, {} urls => cache_correction.len is now {}", cnt, cache_correction.len());
        // info!("cache_fuzzy {:?}", cache_fuzzy);
        // info!("cache_correction {:?}", cache_correction);

        cache_correction_arc = Arc::new(cache_correction);
        cache_fuzzy_arc = Arc::new(cache_fuzzy);
        {
            let mut cx = global_context.write().await;
            cx.documents_state.cache_correction = cache_correction_arc.clone();
            cx.documents_state.cache_fuzzy = cache_fuzzy_arc.clone();
        }
        *cache_dirty_ref = false;
    }
    return (cache_correction_arc, cache_fuzzy_arc)
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
        return vec![fixed.clone()];
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
        let sorted_paths = top_n_records.iter().map(|(path, _)| {
            let mut x = path.clone();
            if let Some(fixed) = (*cache_correction_arc).get(&x) {
                x = fixed.clone();
            }
            x
        }).collect::<Vec<String>>();
        return sorted_paths;
    }

    return vec![];
}

pub fn canonical_path(s: &String) -> PathBuf {
    let mut res = match PathBuf::from(s).canonicalize() {
        Ok(x) => x,
        Err(e) => {
            warn!("canonical_path: {:?} doesn't work: {}", s, e);
            return PathBuf::from(s);
        }
    };
    // info!("WTF: {:?}", res);
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


// FIXME: make sure error printed, not unwrap_or_default
pub async fn get_file_text_from_memory_or_disk(global_context: Arc<ARwLock<GlobalContext>>, file_path: &PathBuf) -> Result<String, String>
{
    if let Some(doc) = global_context.read().await.documents_state.memory_document_map.get(file_path) {
        let doc = doc.read().await;
        if doc.text.is_some() {
            return Ok(doc.text.as_ref().unwrap().to_string());
        }
    }
    read_file_from_disk(&file_path).await.map(|x|x.to_string())
}

impl Document {
    pub fn new(path: &PathBuf) -> Self {
        Self { path: path.clone(), text: None }
    }

    pub async fn update_text_from_disk(&mut self) -> Result<(), String> {
        match read_file_from_disk(&self.path).await {
            Ok(res) => {
                self.text = Some(res);
                return Ok(());
            },
            Err(e) => {
                return Err(e)
            }
        }
    }

    pub async fn get_text_or_read_from_disk(&mut self) -> Result<String, String> {
        if self.text.is_some() {
            return Ok(self.text.as_ref().unwrap().to_string());
        }
        read_file_from_disk(&self.path).await.map(|x|x.to_string())
    }

    pub fn update_text(&mut self, text: &String) {
        self.text = Some(Rope::from_str(text));
    }

    pub fn text_as_string(&self) -> Result<String, String> {
        if let Some(r) = &self.text {
            return Ok(r.to_string());
        }
        return Err(format!("no text loaded in {}", self.path.display()));
    }
}

pub struct DocumentsState {
    pub workspace_folders: Arc<StdMutex<Vec<PathBuf>>>,
    pub workspace_files: Arc<StdMutex<Vec<PathBuf>>>,
    pub jsonl_files: Arc<StdMutex<Vec<PathBuf>>>,
    // document_map on windows: c%3A/Users/user\Documents/file.ext
    // query on windows: C:/Users/user/Documents/file.ext
    pub memory_document_map: HashMap<PathBuf, Arc<ARwLock<Document>>>,   // if a file is open in IDE, and it's outside workspace dirs, it will be in this map and not in workspace_files
    pub cache_dirty: Arc<AMutex<bool>>,
    pub cache_correction: Arc<HashMap<String, String>>,  // map dir3/file.ext -> to /dir1/dir2/dir3/file.ext
    pub cache_fuzzy: Arc<Vec<String>>,                   // slow linear search
    pub fs_watcher: Arc<ARwLock<RecommendedWatcher>>,
}

async fn overwrite_or_create_document(
    global_context: Arc<ARwLock<GlobalContext>>,
    document: Document
) -> (Arc<ARwLock<Document>>, Arc<AMutex<bool>>, bool) {
    let mut cx = global_context.write().await;
    let doc_map = &mut cx.documents_state.memory_document_map;
    if let Some(existing_doc) = doc_map.get_mut(&document.path) {
        *existing_doc.write().await = document;
        (existing_doc.clone(), cx.documents_state.cache_dirty.clone(), false)
    } else {
        let path = document.path.clone();
        let darc = Arc::new(ARwLock::new(document));
        doc_map.insert(path, darc.clone());
        (darc, cx.documents_state.cache_dirty.clone(), true)
    }
}

impl DocumentsState {
    pub async fn new(
        workspace_dirs: Vec<PathBuf>
    ) -> Self {
        let watcher = RecommendedWatcher::new(|_|{}, Default::default()).unwrap();
        Self {
            workspace_folders: Arc::new(StdMutex::new(workspace_dirs)),
            workspace_files: Arc::new(StdMutex::new(Vec::new())),
            jsonl_files: Arc::new(StdMutex::new(Vec::new())),
            memory_document_map: HashMap::new(),
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


pub async fn read_file_from_disk(path: &PathBuf) -> Result<Rope, String> {
    tokio::fs::read_to_string(path).await
        .map(|x|Rope::from_str(&x))
        .map_err(|e| format!("failed to read file {}: {}", crate::nicer_logs::last_n_chars(&path.display().to_string(), 30), e))
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

async fn enqueue_some_docs(
    gcx: Arc<ARwLock<GlobalContext>>,
    docs: &Vec<Document>,
) {
    info!("=> enqueue {} of them", docs.len());
    info!("detected {} modified or added files", docs.len());
    for d in docs.iter().take(5) {
        info!("    added/modified {}", crate::nicer_logs::last_n_chars(&d.path.display().to_string(), 30));
    }
    if docs.len() > 5 {
        info!("    ...");
    }
    let (vec_db_module, ast_module) = {
        let cx = gcx.write().await;
        (cx.vec_db.clone(), cx.ast_module.clone())
    };
    if let Some(ref mut db) = *vec_db_module.lock().await {
        db.vectorizer_enqueue_files(&docs, false).await;
    }
    if let Some(ast) = &ast_module {
        ast.read().await.ast_indexer_enqueue_files(&docs, true).await;
    }
}

pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> i32 {
    let folders: Vec<PathBuf> = gcx.read().await.documents_state.workspace_folders.lock().unwrap().clone();

    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let paths = retrieve_files_by_proj_folders(folders).await;
    info!("enqueue_all_files_from_workspace_folders found {} files => workspace_files", paths.len());
    let newset: HashSet<PathBuf> = paths.iter().cloned().collect();

    let mut documents: Vec<Document> = vec![];
    for d in paths.iter() {
        documents.push(Document { path: d.clone(), text: None });
    }

    let (vec_db_module, ast_module, removed_old) = {
        let cx = gcx.write().await;
        *cx.documents_state.cache_dirty.lock().await = true;
        let workspace_files = &mut cx.documents_state.workspace_files.lock().unwrap();
        let removed_old: HashSet<PathBuf> = workspace_files.iter().filter(|p|!newset.contains(*p)).cloned().collect();
        workspace_files.clear();
        workspace_files.extend(paths);
        (cx.vec_db.clone(), cx.ast_module.clone(), removed_old)
    };
    info!("detected {} deleted files", removed_old.len());
    for p in removed_old.iter().take(5) {
        info!("    deleted {}", crate::nicer_logs::last_n_chars(&p.display().to_string(), 30));
    }
    if removed_old.len() > 5 {
        info!("    ...");
    }
    let full_rebuild = removed_old.len() > 0;

    if let Some(ref mut db) = *vec_db_module.lock().await {
        db.vectorizer_enqueue_files(&documents, true).await;
    }
    if let Some(ast) = &ast_module {
        let x = ast.read().await;
        if full_rebuild {
            x.ast_reset_index().await;
        }
        x.ast_indexer_enqueue_files(&documents, false).await;
    }
    documents.len() as i32
}

pub async fn on_workspaces_init(gcx: Arc<ARwLock<GlobalContext>>) -> i32 {
    enqueue_all_files_from_workspace_folders(gcx.clone()).await
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
    let (_doc_arc, dirty_arc, mark_dirty) = overwrite_or_create_document(gcx.clone(), doc).await;
    if mark_dirty {
        (*dirty_arc.lock().await) = true;
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
        let (doc_arc, dirty_arc, set_mark_dirty) = overwrite_or_create_document(gcx.clone(), doc).await;
        (doc_arc, dirty_arc, set_mark_dirty)
    };

    if mark_dirty {
        (*dirty_arc.lock().await) = true;
    }

    let mut go_ahead = true;
    {
        let is_it_good = is_valid_file(path);
        if is_it_good.is_err() {
            info!("{:?} ignoring changes: {}", path, is_it_good.err().unwrap());
            go_ahead = false;
        }
    }

    let doc = Document { path: doc_arc.read().await.path.clone(), text: None };
    if go_ahead {
        enqueue_some_docs(gcx.clone(), &vec![doc]).await;
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

    let (vec_db_module, ast_module, dirty_arc) = {
        let mut cx = gcx.write().await;
        cx.documents_state.memory_document_map.remove(path);
        (cx.vec_db.clone(), cx.ast_module.clone(), cx.documents_state.cache_dirty.clone())
    };

    (*dirty_arc.lock().await) = true;

    match *vec_db_module.lock().await {
        Some(ref mut db) => db.remove_file(path).await,
        None => {}
    }
    match &ast_module {
        Some(ast) => ast.write().await.remove_file(path).await,
        None => {}
    };
}

pub async fn add_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf)
{
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().push(path.clone());
        let _ = documents_state.fs_watcher.write().await.watch(&path.clone(), RecursiveMode::Recursive);
    }
    let paths = retrieve_files_by_proj_folders(vec![path.clone()]).await;
    let docs: Vec<Document> = paths.into_iter().map(|p| Document { path: p, text: None }).collect();
    enqueue_some_docs(gcx, &docs).await;
}

pub async fn remove_folder(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf)
{
    {
        let documents_state = &mut gcx.write().await.documents_state;
        documents_state.workspace_folders.lock().unwrap().retain(|p| p != path);
        let _ = documents_state.fs_watcher.write().await.unwatch(&path.clone());
    }
    enqueue_all_files_from_workspace_folders(gcx.clone()).await;
}

pub async fn file_watcher_thread(event: Event, gcx: Weak<ARwLock<GlobalContext>>)
{
    async fn on_create_modify(gcx: Weak<ARwLock<GlobalContext>>, event: Event) {
        let mut docs = vec![];
        for p in &event.paths {
            if is_this_inside_blacklisted_dir(&p) {  // important to filter BEFORE canonical_path
                continue;
            }
            let cpath = crate::files_in_workspace::canonical_path(&p.to_string_lossy().to_string());
            docs.push(Document { path: cpath, text: None });
        }
        if docs.is_empty() {
            return;
        }
        info!("EventKind::Create/Modify {} paths", event.paths.len());
        if let Some(gcx) = gcx.upgrade() {
            enqueue_some_docs(gcx, &docs).await;
        }
    }

    async fn on_remove(gcx: Weak<ARwLock<GlobalContext>>, event: Event) {
        let mut never_mind = true;
        for p in &event.paths {
            never_mind &= is_this_inside_blacklisted_dir(&p);
            // let cpath = crate::files_in_workspace::canonical_path(&p.to_string_lossy().to_string());
            // nothing we can do with cpath
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
