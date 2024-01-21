use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use log::{info, error};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::RwLock as ARwLock;

use crate::global_context::GlobalContext;

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


async fn parse_jsonl(path: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let file = File::open(path).await.map_err(|_| format!("File not found: {:?}", path))?;
    let reader = BufReader::new(file);
    let base_path = path.parent().or(Some(Path::new("/"))).unwrap().to_path_buf();

    let mut lines = reader.lines();
    let mut paths = Vec::new();

    while let Some(line) = lines.next_line().await.transpose() {
        let line = line.map_err(|_| "Error reading line".to_string())?;
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if value.is_object() {
                if let Some(filename) = value.get("path").and_then(|v| v.as_str()) {
                    paths.push(base_path.join(filename));
                }
            }
        }
    }

    Ok(paths)
}

pub async fn file_watcher_task(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    let (mut watcher, mut rx) = make_async_watcher().expect("Failed to make file watcher");
    let maybe_path = global_context.read().await.cmdline.files_set_path.clone();
    if maybe_path.is_empty() {
        info!("files_set_path is empty: no files to watch");
        return;
    }
    let path = PathBuf::from(maybe_path);
    let load_data = || async {
        let filenames_vec = match parse_jsonl(&path).await {
            Ok(data) => data,
            Err(_) => {
                info!("invalid jsonl file: {:?}", path);
                vec![]
            }
        };
        match *global_context.read().await.vec_db.lock().await {
            Some(ref mut db) => db.add_or_update_files(filenames_vec, true).await,
            None => {}
        };
    };

    if watcher.watch(path.as_ref(), RecursiveMode::Recursive).is_err() {
        error!("file watcher {:?} failed to start watching", path);
        return;
    }
    load_data().await;
    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                match event.kind {
                    EventKind::Any => {}
                    EventKind::Access(_) => {}
                    EventKind::Create(_) => {
                        info!("file {:?} was created", path)
                    }
                    EventKind::Modify(_) => {
                        load_data().await;
                    }
                    EventKind::Remove(_) => {
                        info!("file {:?} was removed", path)
                        // TODO: should we remove everything inside the database?
                    }
                    EventKind::Other => {}
                }
            }
            Err(e) => info!("file watch error: {:?}", e),
        }
    }
}
