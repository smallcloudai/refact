use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tracing::info;

use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::handler::VecDBHandlerRef;
use crate::fetch_embedding::try_get_embedding;
use crate::vecdb::structs::{Record, SplitResult, VecDbStatus, VecDbStatusRef};

#[derive(Debug)]
pub struct FileVectorizerService {
    update_request_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    output_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    vecdb_handler: VecDBHandlerRef,
    status: VecDbStatusRef,
    cooldown_secs: u64,
    splitter_window_size: usize,
    splitter_soft_limit: usize,

    model_name: String,
    api_key: String,
    endpoint_embeddings_style: String,
    endpoint_template: String,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    out_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    _status: VecDbStatusRef,
    cooldown_secs: u64,
) {
    let mut last_updated: HashMap<PathBuf, SystemTime> = HashMap::new();
    loop {
        let (path_maybe, _unprocessed_files_count) = {
            let mut queue_locked = update_request_queue.lock().await;
            if !queue_locked.is_empty() {
                (Some(queue_locked.pop_front().unwrap()), queue_locked.len())
            } else {
                (None, 0)
            }
        };

        if let Some(path) = path_maybe {
            last_updated.insert(path, SystemTime::now());
        }

        let mut paths_to_process: Vec<PathBuf> = Vec::new();
        for (path, time) in &last_updated {
            if time.elapsed().unwrap().as_secs() > cooldown_secs {
                paths_to_process.push(path.clone());
            }
        }
        for path in paths_to_process {
            last_updated.remove(&path);
            out_queue.lock().await.push_back(path);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}


async fn vectorize_thread(
    queue: Arc<Mutex<VecDeque<PathBuf>>>,
    vecdb_handler_ref: VecDBHandlerRef,
    status: VecDbStatusRef,
    splitter_window_size: usize,
    splitter_soft_limit: usize,

    model_name: String,
    api_key: String,
    endpoint_embeddings_style: String,
    endpoint_template: String,

    max_concurrent_tasks: usize,
) {
    let file_splitter = FileSplitter::new(splitter_window_size, splitter_soft_limit);
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));

    loop {
        let (path_maybe, unprocessed_files_count) = {
            let mut queue_locked = queue.lock().await;
            if !queue_locked.is_empty() {
                (Some(queue_locked.pop_front().unwrap()), queue_locked.len())
            } else {
                (None, 0)
            }
        };
        status.lock().await.unprocessed_files_count = unprocessed_files_count;
        let path = {
            match path_maybe {
                Some(path) => path,
                None => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    continue;
                }
            }
        };

        let split_data = match file_splitter.split(&path).await {
            Ok(data) => data,
            Err(_) => { continue }
        };

        let mut vecdb_handler = vecdb_handler_ref.lock().await;
        let mut split_data_filtered: Vec<SplitResult> = split_data
            .iter()
            .filter(|x| !vecdb_handler.contains(&x.window_text_hash))
            .cloned() // Clone to avoid borrowing issues
            .collect();
        split_data_filtered = vecdb_handler.try_add_from_cache(split_data_filtered).await;
        drop(vecdb_handler);

        let last_30_chars: String = path.display().to_string().chars().rev().take(30).collect::<String>().chars().rev().collect();
        info!("...{} embeddings todo/total {}/{}", last_30_chars, split_data_filtered.len(), split_data.len());

        // TODO: replace with a batched call?
        let join_handles: Vec<_> = split_data_filtered.into_iter().map(|x| {
            let model_name_clone = model_name.clone();
            let api_key_clone = api_key.clone();
            let endpoint_embeddings_style_clone = endpoint_embeddings_style.clone();
            let endpoint_template_clone = endpoint_template.clone();

            let semaphore_clone = Arc::clone(&semaphore);
            tokio::spawn(async move {
                let _permit = match semaphore_clone.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return None;
                    }
                };

                let result = try_get_embedding(
                    &endpoint_embeddings_style_clone,
                    &model_name_clone,
                    &endpoint_template_clone,
                    x.window_text.clone(),
                    &api_key_clone,
                    3,
                ).await;

                drop(_permit);
                Some((x, result))
            })
        }).collect();

        let mut records = vec![];

        for handle in join_handles {
            if let Some((data_res, result_mb)) = handle.await.unwrap() {
                match result_mb {
                    Ok(result) => {
                        let now = SystemTime::now();

                        records.push(
                            Record {
                                vector: Some(result),
                                window_text: data_res.window_text,
                                window_text_hash: data_res.window_text_hash,
                                file_path: data_res.file_path,
                                start_line: data_res.start_line,
                                end_line: data_res.end_line,
                                time_added: SystemTime::now(),
                                model_name: model_name.clone(),
                                distance: -1.0,
                                used_counter: 0,
                                time_last_used: now,
                            }
                        );
                    }
                    Err(e) => {
                        info!("Error retrieving embeddings for {}: {}", data_res.file_path.to_str().unwrap(), e);
                    }
                }
            }
        }
        match vecdb_handler_ref.lock().await.add_or_update(records, true).await {
            Err(e) => {
                info!("Error adding/updating records in VecDB: {}", e);
            }
            _ => {}
        }
    }
}

async fn cleanup_thread(vecdb_handler: VecDBHandlerRef) {
    loop {
        {
            let mut vecdb = vecdb_handler.lock().await;
            let _ = vecdb.cleanup_old_records().await;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2 * 3600)).await;
    }
}

impl FileVectorizerService {
    pub async fn new(
        vecdb_handler: VecDBHandlerRef,
        cooldown_secs: u64,
        splitter_window_size: usize,
        splitter_soft_limit: usize,

        model_name: String,
        api_key: String,
        endpoint_embeddings_style: String,
        endpoint_template: String,
    ) -> Self {
        let update_request_queue = Arc::new(Mutex::new(VecDeque::new()));
        let output_queue = Arc::new(Mutex::new(VecDeque::new()));
        let status = Arc::new(Mutex::new(
            VecDbStatus {
                unprocessed_files_count: 0,
                requests_made_since_start: 0,
                db_size: 0,
                db_cache_size: 0,
            }
        ));
        FileVectorizerService {
            update_request_queue: update_request_queue.clone(),
            output_queue: output_queue.clone(),
            vecdb_handler: vecdb_handler.clone(),
            status: status.clone(),
            cooldown_secs,
            splitter_window_size,
            splitter_soft_limit,

            model_name,
            api_key,
            endpoint_embeddings_style,
            endpoint_template,
        }
    }

    pub async fn start_background_tasks(&self) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                self.status.clone(),
                self.cooldown_secs,
            )
        );

        let retrieve_thread_handle = tokio::spawn(
            vectorize_thread(
                self.output_queue.clone(),
                self.vecdb_handler.clone(),
                self.status.clone(),
                self.splitter_window_size,
                self.splitter_soft_limit,

                self.model_name.clone(),
                self.api_key.clone(),
                self.endpoint_embeddings_style.clone(),
                self.endpoint_template.clone(),

                4,
            )
        );

        let cleanup_thread_handle = tokio::spawn(
            cleanup_thread(
                self.vecdb_handler.clone()
            )
        );

        return vec![cooldown_queue_join_handle, retrieve_thread_handle, cleanup_thread_handle];
    }

    pub async fn process_file(&self, path: PathBuf, force: bool) {
        if !force {
            self.update_request_queue.lock().await.push_back(path);
        } else {
            self.output_queue.lock().await.push_back(path);
        }
    }

    pub async fn process_files(&self, paths: Vec<PathBuf>, force: bool) {
        if !force {
            self.update_request_queue.lock().await.extend(paths);
        } else {
            self.output_queue.lock().await.extend(paths);
        }
    }

    pub async fn status(&self) -> Result<VecDbStatus, String> {
        let mut status = self.status.lock().await.clone();
        status.db_size = match self.vecdb_handler.lock().await.size().await {
            Ok(res) => res,
            Err(err) => return Err(err)
        };
        status.db_cache_size = match self.vecdb_handler.lock().await.cache_size().await {
            Ok(res) => res,
            Err(err) => return Err(err)
        };
        Ok(status)
    }
}
