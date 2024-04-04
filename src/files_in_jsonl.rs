use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use tracing::{info, error};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::RwLock as ARwLock;
use crate::files_in_workspace::{Document, read_file_from_disk};

use crate::global_context::GlobalContext;


pub async fn enqueue_all_docs_from_jsonl(gcx: Arc<ARwLock<GlobalContext>>) {
    let docs = docs_in_jsonl(gcx.clone()).await;
    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    let mut documents = vec![];
    for d in docs {
        documents.push(d.read().await.clone())
    }
    match &ast_module {
        Some(ast) => ast.read().await.ast_indexer_enqueue_files(&documents, true).await,
        None => {},
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&documents, false).await,
        None => {},
    };
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

pub async fn docs_in_jsonl(global_context: Arc<ARwLock<GlobalContext>>) -> Vec<Arc<ARwLock<Document>>> {
    let mut docs = vec![];
    for doc in global_context.read().await.documents_state.document_map.values() {
        if doc.read().await.in_jsonl {
            docs.push(doc.clone());
        }
    }
    docs
}

pub async fn files_in_jsonl_w_path(files_jsonl_path: &String) -> Vec<PathBuf> {
    match parse_jsonl(files_jsonl_path).await {
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


async fn add_or_set_in_jsonl(
    gcx: Arc<ARwLock<GlobalContext>>,
    paths: Vec<PathBuf>,
) {
    let mut cx = gcx.write().await;
    let docs_map = &mut cx.documents_state.document_map;
    for p in paths.iter() {
        let text = &read_file_from_disk(p).await.map(|x|x.to_string()).unwrap_or_default();
        if let Some(doc) = docs_map.get_mut(p) {
            doc.write().await.in_jsonl = true;
            doc.write().await.update_text(text);
        } else {
            let mut doc = Document::new(p, None);
            doc.in_jsonl = true;
            doc.update_text(text);
            docs_map.insert(p.clone(), Arc::new(ARwLock::new(doc)));
        }
    }
    *cx.documents_state.cache_dirty.lock().await = true;
}

pub async fn reload_if_jsonl_changes_background_task(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    async fn on_modify(gcx: Arc<ARwLock<GlobalContext>>, files_jsonl_path: &String) {
        let paths = files_in_jsonl_w_path(&files_jsonl_path).await;
        add_or_set_in_jsonl(gcx.clone(), paths).await;
        enqueue_all_docs_from_jsonl(gcx.clone()).await;
    }
    let (mut watcher, mut rx) = make_async_watcher().expect("Failed to make file watcher");
    let files_jsonl_path = gcx.read().await.cmdline.files_jsonl_path.clone();
    on_modify(gcx.clone(), &files_jsonl_path).await;
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
                        on_modify(gcx.clone(), &files_jsonl_path).await;
                    }
                    EventKind::Remove(_) => {
                        info!("files_jsonl_path {:?} was removed", files_jsonl_path);
                        // TODO: do something sensible?
                    }
                    EventKind::Other => {}
                }
            }
            Err(e) => info!("file watch error: {:?}", e),
        }
    }
}
