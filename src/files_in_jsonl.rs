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

use crate::global_context::GlobalContext;


pub async fn enqueue_all_files_from_jsonl(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let files_jsonl_path = gcx.clone().read().await.cmdline.files_jsonl_path.clone();
    let files = match parse_jsonl(&files_jsonl_path).await {
        Ok(data) => data,
        Err(e) => {
            info!("invalid jsonl file {:?}: {:?}", files_jsonl_path, e);
            vec![]
        }
    };
    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&files).await,
        None => {},
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&files, false).await,
        None => {},
    };
}

pub async fn parse_jsonl(path: &String) -> Result<Vec<PathBuf>, String> {
    if path.is_empty() {
        return Ok(vec![]);
    }
    let file = File::open(path).await.map_err(|_| format!("File not found: {:?}", path))?;
    let reader = BufReader::new(file);
    let base_path = PathBuf::from(path).parent().or(Some(Path::new("/"))).unwrap().to_path_buf();

    let mut lines = reader.lines();
    let mut paths = Vec::new();

    while let Some(line) = lines.next_line().await.transpose() {
        let line = line.map_err(|_| "Error reading line".to_string())?;
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if value.is_object() {
                if let Some(filename) = value.get("path").and_then(|v| v.as_str()) {
                    // TODO: join, why it's there?
                    paths.push(base_path.join(filename));
                }
            }
        }
    }
    Ok(paths)
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
) -> () {
    let (mut watcher, mut rx) = make_async_watcher().expect("Failed to make file watcher");
    let files_jsonl_path = gcx.read().await.cmdline.files_jsonl_path.clone();
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
                        info!("files list {:?} was created", files_jsonl_path);
                    }
                    EventKind::Modify(_) => {
                        info!("files list {:?} was modified", files_jsonl_path);
                        enqueue_all_files_from_jsonl(gcx.clone()).await;
                    }
                    EventKind::Remove(_) => {
                        info!("files fist {:?} was removed", files_jsonl_path);
                        // TODO: do something sensible?
                    }
                    EventKind::Other => {}
                }
            }
            Err(e) => info!("file watch error: {:?}", e),
        }
    }
}
