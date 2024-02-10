use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::SystemTime;
use std::ops::Div;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::info;

use crate::vecdb::file_splitter::FileSplitter;
use crate::vecdb::handler::VecDBHandler;
use crate::fetch_embedding::try_get_embedding;
use crate::vecdb::structs::{Record, SplitResult, VecDbStatus, VecdbConstants};

#[derive(Debug)]
pub struct FileVectorizerService {
    update_request_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    output_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    vecdb_handler: Arc<AMutex<VecDBHandler>>,
    status: Arc<AMutex<VecDbStatus>>,
    constants: VecdbConstants,
    api_key: String,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    out_queue: Arc<AMutex<VecDeque<PathBuf>>>,
    _status: Arc<AMutex<VecDbStatus>>,
    cooldown_secs: u64,
) {
    // This function delays vectorization of a file, until mtime is at least cooldown_secs old.
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
            info!("cooldown_queue_thread: {} files to process, {} files too new", stat_proceed, stat_too_new);
        }
        for path in paths_to_process {
            last_updated.remove(&path);
            out_queue.lock().await.push_back(path);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}


async fn vectorize_thread(
    client: Arc<AMutex<reqwest::Client>>,
    queue: Arc<AMutex<VecDeque<PathBuf>>>,
    vecdb_handler_ref: Arc<AMutex<VecDBHandler>>,
    status: Arc<AMutex<VecDbStatus>>,
    constants: VecdbConstants,
    api_key: String,
    max_concurrent_tasks: usize,
) {
    let file_splitter = FileSplitter::new(constants.splitter_window_size, constants.splitter_soft_limit);
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));
    let mut reported_unprocessed: usize = 0;
    let mut reported_vecdb_complete: bool = false;

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
        status.lock().await.unprocessed_files_count = unprocessed_files_count;
        reported_vecdb_complete &= unprocessed_files_count==0;
        let path = {
            match path_maybe {
                Some(path) => path,
                None => {
                    // No files left to process
                    if !reported_vecdb_complete {
                        let t0 = std::time::Instant::now();
                        vecdb_handler_ref.lock().await.update_indexed_file_paths().await;
                        info!("update_indexed_file_paths: it took {:.3}s", t0.elapsed().as_secs_f64());

                        reported_vecdb_complete = true;
                        // For now we do not create index 'cause it hurts quality of retrieval
                        // info!("VECDB Creating index");
                        // match vecdb_handler_ref.lock().await.create_index().await {
                        //     Ok(_) => info!("VECDB CREATED INDEX"),
                        //     Err(err) => info!("VECDB Error creating index: {}", err)
                        // }
                        write!(std::io::stderr(), "VECDB COMPLETE\n").unwrap();
                        info!("VECDB COMPLETE"); // you can see stderr "VECDB COMPLETE" sometimes faster vs logs
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
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

        let last_30_chars = crate::nicer_logs::last_n_chars(&path.display().to_string(), 30);
        info!("embeddings {} todo/total {}/{}", last_30_chars, split_data_filtered.len(), split_data.len());

        // TODO: replace with a batched call?
        let join_handles: Vec<_> = split_data_filtered.into_iter().map(|x| {
            let model_name_clone = constants.model_name.clone();
            let api_key_clone = api_key.clone();
            let endpoint_embeddings_style_clone = constants.endpoint_embeddings_style.clone();
            let endpoint_template_clone = constants.endpoint_embeddings_template.clone();
            let status_clone = Arc::clone(&status);
            let client_clone = Arc::clone(&client);

            let semaphore_clone = Arc::clone(&semaphore);
            tokio::spawn(async move {
                let _permit = match semaphore_clone.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return None;
                    }
                };

                let result = try_get_embedding(
                    client_clone,
                    &endpoint_embeddings_style_clone,
                    &model_name_clone,
                    &endpoint_template_clone,
                    x.window_text.clone(),
                    &api_key_clone,
                    1,
                ).await;
                status_clone.lock().await.requests_made_since_start += 1;

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
                                time_added: now,
                                model_name: constants.model_name.clone(),
                                distance: -1.0,
                                used_counter: 0,
                                time_last_used: now,
                            }
                        );
                    }
                    Err(e) => {
                        info!("Error retrieving embeddings for {}: {}", data_res.file_path.to_str().unwrap(), e);
                        // queue.lock().await.push_back(data_res.file_path);  // push it back again
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

async fn cleanup_thread(vecdb_handler: Arc<AMutex<VecDBHandler>>) {
    loop {
        {
            let mut vecdb = vecdb_handler.lock().await;
            let _ = vecdb.cleanup_old_records().await;
            // By the time we do not create index 'cause it hurts quality of retrieval
            // let _ = vecdb.create_index().await;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2 * 3600)).await;
    }
}

impl FileVectorizerService {
    pub async fn new(
        vecdb_handler: Arc<AMutex<VecDBHandler>>,
        constants: VecdbConstants,
        api_key: String,
    ) -> Self {
        let update_request_queue = Arc::new(AMutex::new(VecDeque::new()));
        let output_queue = Arc::new(AMutex::new(VecDeque::new()));
        let status = Arc::new(AMutex::new(
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
            constants,
            api_key,
        }
    }

    pub async fn start_background_tasks(&self, vecdb_client: Arc<AMutex<reqwest::Client>>) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                self.status.clone(),
                self.constants.cooldown_secs,
            )
        );

        let retrieve_thread_handle = tokio::spawn(
            vectorize_thread(
                vecdb_client.clone(),
                self.output_queue.clone(),
                self.vecdb_handler.clone(),
                self.status.clone(),
                self.constants.clone(),
                self.api_key.clone(),
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
        info!("adding single file");
        if !force {
            self.update_request_queue.lock().await.push_back(path);
        } else {
            self.output_queue.lock().await.push_back(path);
        }
    }

    pub async fn process_files(&self, paths: Vec<PathBuf>, force: bool) {
        info!("adding {} files", paths.len());
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
            Err(err) => return Err(err.to_string())
        };
        Ok(status)
    }
}
