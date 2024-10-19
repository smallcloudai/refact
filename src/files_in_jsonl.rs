use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::channel::mpsc::{channel, Receiver};
use futures::{SinkExt, StreamExt};
use tracing::{info, error};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::RwLock as ARwLock;

use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::ast::ast_indexer_thread::ast_indexer_enqueue_files;


pub async fn enqueue_all_docs_from_jsonl(
    gcx: Arc<ARwLock<GlobalContext>>,
    paths: Vec<PathBuf>,
    force: bool,
    vecdb_only: bool,
) {
    if paths.is_empty() {
        return;
    }
    let mut docs: Vec<Document> = vec![];
    for d in paths.iter() {
        docs.push(Document { doc_path: d.clone(), doc_text: None });
    }
    let (vec_db_module, ast_service) = {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        let gcx_locked = gcx.write().await;
        *gcx_locked.documents_state.cache_dirty.lock().await = now;
        let jsonl_files = &mut gcx_locked.documents_state.jsonl_files.lock().unwrap();
        jsonl_files.clear();
        jsonl_files.extend(paths);
        #[cfg(feature="vecdb")]
        let vec_db_module = gcx_locked.vec_db.clone();
        #[cfg(not(feature="vecdb"))]
        let vec_db_module = false;
        (vec_db_module, gcx_locked.ast_service.clone())
    };
    if let Some(ast) = &ast_service {
        if !vecdb_only {
            let cpaths: Vec<String> = docs.iter().map(|doc| doc.doc_path.to_string_lossy().to_string()).collect();
            ast_indexer_enqueue_files(ast.clone(), cpaths, force).await;
        }
    }
    #[cfg(feature="vecdb")]
    match *vec_db_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, false).await,
        None => {},
    };
    #[cfg(not(feature="vecdb"))]
    let _ = vec_db_module;
}

pub async fn enqueue_all_docs_from_jsonl_but_read_first(
    gcx: Arc<ARwLock<GlobalContext>>,
    force: bool,
    vecdb_only: bool,
) {
    let paths = read_the_jsonl(gcx.clone()).await;
    enqueue_all_docs_from_jsonl(gcx.clone(), paths, force, vecdb_only).await;
}

async fn parse_jsonl(jsonl_path: &String) -> Result<Vec<PathBuf>, String> {
    if jsonl_path.is_empty() {
        return Ok(vec![]);
    }
    let file = File::open(jsonl_path).await.map_err(|_| format!("File not found: {:?}", jsonl_path))?;
    let reader = BufReader::new(file);
    let base_path = PathBuf::from(jsonl_path).parent().or(Some(Path::new("/"))).unwrap().to_path_buf();

    let mut lines = reader.lines();

    let mut paths = Vec::new();
    while let Some(line) = lines.next_line().await.transpose() {
        let line = line.map_err(|_| "Error reading line".to_string())?;
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if value.is_object() {

                if let Some(filename) = value.get("path").and_then(|v| v.as_str()) {
                    // TODO: join, why it's there?
                    let path = base_path.join(filename);
                    paths.push(path);
                }
            }
        }
    }
    Ok(paths)
}

pub async fn read_the_jsonl(gcx: Arc<ARwLock<GlobalContext>>) -> Vec<PathBuf> {
    let files_jsonl_path = gcx.read().await.cmdline.files_jsonl_path.clone();
    match parse_jsonl(&files_jsonl_path).await {
        Ok(docs) => docs,
        Err(e) => {
            info!("invalid jsonl file {:?}: {:?}", files_jsonl_path, e);
            vec![]
        }
    }
}

fn make_async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

pub async fn reload_if_jsonl_changes_background_task(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    async fn on_modify(gcx: Arc<ARwLock<GlobalContext>>) {
        enqueue_all_docs_from_jsonl_but_read_first(gcx.clone(), false, false).await;
    }
    let (mut watcher, mut rx) = make_async_watcher().expect("Failed to make file watcher");
    let files_jsonl_path = gcx.read().await.cmdline.files_jsonl_path.clone();
    on_modify(gcx.clone()).await;
    if watcher.watch(&PathBuf::from(files_jsonl_path.clone()), RecursiveMode::Recursive).is_err() {
        error!("file watcher {:?} failed to start watching", files_jsonl_path);
        return;
    }
    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                match event.kind {
                    EventKind::Any => {}
                    EventKind::Access(_) => {}
                    EventKind::Create(_) => {
                        info!("files_jsonl_path {:?} was created", files_jsonl_path);
                    }
                    EventKind::Modify(_) => {
                        info!("files_jsonl_path {:?} was modified", files_jsonl_path);
                        enqueue_all_docs_from_jsonl(gcx.clone(), vec![], false, false).await;
                    }
                    EventKind::Remove(_) => {
                        info!("files_jsonl_path {:?} was removed", files_jsonl_path);
                        enqueue_all_docs_from_jsonl(gcx.clone(), vec![], false, false).await;
                    }
                    EventKind::Other => {}
                }
            }
            Err(e) => info!("file watch error: {:?}", e),
        }
    }
}
