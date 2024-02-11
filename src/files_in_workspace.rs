use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use ropey::Rope;

use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context;
use crate::telemetry;
use crate::vecdb::file_filter;



#[derive(Debug)]
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

pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    let folders: Vec<PathBuf> = {
        let cx_locked = gcx.read().await;
        let x = cx_locked.documents_state.workspace_folders.lock().unwrap().clone();
        x
    };
    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let files = file_filter::retrieve_files_by_proj_folders(folders).await;
    info!("enqueue_all_files_from_workspace_folders found {} files", files.len());
    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&files).await,
        None => {
            info!("ast_module is None");
        },
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&files, false).await,
        None => {},
    };
    files.len() as i32
}

pub async fn on_workspaces_init(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    // TODO: this will not work when files change. Need a real file watcher.
    enqueue_all_files_from_workspace_folders(gcx.clone()).await
}

pub async fn on_did_open(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    fpath: &String,
    text: &String,
    language_id: &String,
) {
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.documents_state.document_map;
    let rope = ropey::Rope::from_str(&text);
    let mut document_map_locked = document_map.write().await;
    *document_map_locked
        .entry(fpath.clone())
        .or_insert(Document::new("unknown".to_owned(), Rope::new())) = Document::new(language_id.clone(), rope);
    let last_30_chars: String = crate::nicer_logs::last_n_chars(fpath, 30);
    info!("opened {}", last_30_chars);
}

pub async fn on_did_change(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    fpath: &String,
    text: &String,
) {
    let t0 = Instant::now();
    {
        let gcx_locked = gcx.read().await;
        let document_map = &gcx_locked.documents_state.document_map;
        let rope = ropey::Rope::from_str(&text);
        let mut document_map_locked = document_map.write().await;
        let doc = document_map_locked
            .entry(fpath.clone())
            .or_insert(Document::new("unknown".to_owned(), Rope::new()));
        doc.text = rope;
    }
    // let binding = global_context.read().await;
    // match *binding.vec_db.lock().await {
    //     Some(ref mut db) => db.vectorizer_enqueue_files(&vec![path.clone()], false).await,
    //     None => {}
    // };
    // match *binding.ast_module.lock().await {
    //     Some(ref mut ast) => ast.ast_indexer_enqueue_files(&vec![path.clone()]).await,
    //     None => {}
    // };
    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        fpath,
        text,
    ).await;
    let last_30_chars: String = crate::nicer_logs::last_n_chars(fpath, 30);
    info!("changed {}, total time {:.3}s", last_30_chars, t0.elapsed().as_secs_f32());
}
