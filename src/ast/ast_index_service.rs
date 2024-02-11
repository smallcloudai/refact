use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::ops::Div;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::Mutex as AMutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::ast::ast_index::AstIndex;

#[derive(Debug)]
pub struct AstIndexService {
    update_request_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    output_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    ast_index: Arc<AMutex<AstIndex>>,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    out_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    cooldown_secs: u64,
) {
    // TODO: remove, don't need cooldown for AST
    let mut last_updated: HashMap<PathBuf, SystemTime> = HashMap::new();
    loop {
        let (path_maybe, _unprocessed_files_count) = {
            let mut queue_locked = update_request_queue.lock().await;
            let queue_len = queue_locked.len();
            if !queue_locked.is_empty() {
                (Some(queue_locked.pop_front().unwrap()), queue_len)
            } else {
                (None, 0)
            }
        };

        if let Some(path) = path_maybe {
            last_updated.insert(path, SystemTime::now());
        }

        let mut paths_to_process: Vec<PathBuf> = Vec::new();
        let mut stat_too_new = 0;
        let mut stat_proceed = 0;
        for (path, time) in &last_updated {
            if time.elapsed().unwrap().as_secs() > cooldown_secs {
                paths_to_process.push(path.clone());
                stat_proceed += 1;
            } else {
                stat_too_new += 1;
            }
        }
        if stat_proceed > 0 || stat_too_new > 0 {
            info!("{} files to process, {} files too new", stat_proceed, stat_too_new);
        }
        for path in paths_to_process {
            last_updated.remove(&path);
            out_queue.lock().await.push_back(path);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}


async fn ast_indexer_thread(
    queue: Arc<AMutex<VecDeque<PathBuf>>>,
    ast_index: Arc<AMutex<AstIndex>>,
) {
    let mut reported_unprocessed: usize = 0;
    let mut reported_astindex_complete: bool = false;

    loop {
        let (path_maybe, unprocessed_files_count) = {
            let mut queue_locked = queue.lock().await;
            let queue_len = queue_locked.len();
            if queue_len > 0 {
                (Some(queue_locked.pop_front().unwrap()), queue_len)
            } else {
                (None, 0)
            }
        };
        if (unprocessed_files_count + 99).div(100) != (reported_unprocessed + 99).div(100) {
            info!("have {} unprocessed files", unprocessed_files_count);
            reported_unprocessed = unprocessed_files_count;
        }
        reported_astindex_complete &= unprocessed_files_count == 0;
        let path = {
            match path_maybe {
                Some(path) => path,
                None => {
                    if !reported_astindex_complete {
                        reported_astindex_complete = true;
                        write!(
                            std::io::stderr(),
                            "AST index building complete\n"
                        ).unwrap();
                        info!("AST index building complete");
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                    continue;
                }
            }
        };
        match ast_index.lock().await.add_or_update(&path).await {
            Err(e) => {
                info!("Error adding/updating records in AST index: {}", e);
            }
            Ok(definitions_vector) => {
                let last_30_chars = crate::nicer_logs::last_n_chars(&path.display().to_string(), 30);
                info!("parse {}, added {} definitions", last_30_chars, definitions_vector.len());
            }
        }
    }
}

const COOLDOWN_SECS: u64 = 20;

impl AstIndexService {
    pub fn init(
        ast_index: Arc<AMutex<AstIndex>>
    ) -> Self {
        let update_request_queue = Arc::new(AMutex::new(VecDeque::new()));
        let output_queue = Arc::new(AMutex::new(VecDeque::new()));
        AstIndexService {
            update_request_queue: update_request_queue.clone(),
            output_queue: output_queue.clone(),
            ast_index: ast_index.clone(),
        }
    }

    pub async fn ast_start_background_tasks(&mut self) -> Vec<JoinHandle<()>> {
        // TODO: don't need cooldown for AST
        // TODO: read file text from memory, text can be found in files_in_workspace
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                COOLDOWN_SECS,
            )
        );
        let indexer_handle = tokio::spawn(
            ast_indexer_thread(
                self.output_queue.clone(),
                self.ast_index.clone(),
            )
        );
        return vec![cooldown_queue_join_handle, indexer_handle];
    }

    pub async fn ast_indexer_enqueue_files(&self, paths: &Vec<PathBuf>, force: bool) {
        info!("Adding to AST index {} files", paths.len());
        if !force {
            self.update_request_queue.lock().await.extend(paths.clone());
        } else {
            self.output_queue.lock().await.extend(paths.clone());
        }
    }
}
