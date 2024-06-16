use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::ops::Div;
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use tokenizers::Tokenizer;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{Mutex as AMutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{info, warn};

use crate::ast::file_splitter::AstBasedFileSplitter;
use crate::fetch_embedding::get_embedding_with_retry;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::handler::VecDBHandler;
use crate::vecdb::structs::{Record, SplitResult, VecdbConstants, VecDbStatus};


#[derive(Debug)]
pub struct FileVectorizerService {
    update_request_queue: Arc<AMutex<VecDeque<Document>>>,
    output_queue: Arc<AMutex<VecDeque<Document>>>,
    vecdb_handler: Arc<AMutex<VecDBHandler>>,
    status: Arc<AMutex<VecDbStatus>>,
    constants: VecdbConstants,
    api_key: String,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<Document>>>,
    out_queue: Arc<AMutex<VecDeque<Document>>>,
    _status: Arc<AMutex<VecDbStatus>>,
    cooldown_secs: u64,
) {
    // This function delays vectorization of a file, until mtime is at least cooldown_secs old.
    let mut last_updated: HashMap<Document, SystemTime> = HashMap::new();
    loop {
        let mut docs: Vec<Document> = Vec::new();
        {
            let mut queue_locked = update_request_queue.lock().await;
            for _ in 0..queue_locked.len() {
                if let Some(doc) = queue_locked.pop_front() {
                    docs.push(doc);
                }
            }
        }

        let current_time = SystemTime::now();
        for doc in docs {
            last_updated.insert(doc, current_time);
        }

        let mut docs_to_process: Vec<Document> = Vec::new();
        let mut stat_too_new = 0;
        let mut stat_proceed = 0;
        for (doc, time) in &last_updated {
            if time.elapsed().unwrap().as_secs() > cooldown_secs {
                docs_to_process.push(doc.clone());
                stat_proceed += 1;
            } else {
                stat_too_new += 1;
            }
        }
        if stat_proceed > 0 || stat_too_new > 0 {
            info!("{} files to process, {} files too new", stat_proceed, stat_too_new);
        }
        for doc in docs_to_process {
            last_updated.remove(&doc);
            out_queue.lock().await.push_back(doc);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn vectorize_batch_from_q(
    embed_q: &mut Vec<SplitResult>,
    status: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<reqwest::Client>>,
    constants: &VecdbConstants,
    api_key: &String,
    vecdb_handler_ref: Arc<AMutex<VecDBHandler>>,
    #[allow(non_snake_case)]
    B: usize,
) -> Result<(), String> {
    let batch = embed_q.drain(..B.min(embed_q.len())).collect::<Vec<_>>();
    let t0 = Instant::now();

    let batch_result = get_embedding_with_retry(
        client.clone(),
        &constants.endpoint_embeddings_style.clone(),
        &constants.model_name.clone(),
        &constants.endpoint_embeddings_template.clone(),
        batch.iter().map(|x|x.window_text.clone()).collect(),
        api_key,
        1,
    ).await?;

    if batch_result.len() != batch.len() {
        return Err(format!("vectorize: batch_result.len() != batch.len(): {} vs {}", batch_result.len(), batch.len()));
    }

    {
        let mut status_locked = status.lock().await;
        status_locked.requests_made_since_start += 1;
        status_locked.vectors_made_since_start += batch_result.len();
    }

    let mut records = vec![];
    let now = SystemTime::now();
    for (i, data_res) in batch.iter().enumerate() {
        records.push(
            Record {
                vector: Some(batch_result[i].clone()),
                window_text: data_res.window_text.clone(),
                window_text_hash: data_res.window_text_hash.clone(),
                file_path: data_res.file_path.clone(),
                start_line: data_res.start_line,
                end_line: data_res.end_line,
                time_added: now,
                model_name: constants.model_name.clone(),
                distance: -1.0,
                used_counter: 0,
                time_last_used: now,
                usefulness: 0.0,
            }
        );
    }

    if records.len() > 0 {
        info!("embeddings got {} records in {}ms", records.len(), t0.elapsed().as_millis());
        match vecdb_handler_ref.lock().await.add_or_update(records, true).await {
            Err(e) => {
                warn!("Error adding/updating records in VecDB: {}", e);
            }
            _ => {}
        }
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;  // be nice to the server: up to 60 requests per minute

    Ok(())
}

async fn vectorize_thread(
    client: Arc<AMutex<reqwest::Client>>,
    queue: Arc<AMutex<VecDeque<Document>>>,
    vecdb_handler_ref: Arc<AMutex<VecDBHandler>>,
    status: Arc<AMutex<VecDbStatus>>,
    constants: VecdbConstants,
    api_key: String,
    tokenizer: Arc<StdRwLock<Tokenizer>>,
    gcx_weak: Weak<RwLock<GlobalContext>>,
) {
    const B: usize = 64;

    let mut files_total: usize = 0;
    let mut reported_unprocessed: usize = 0;
    let mut reported_vecdb_complete: bool = false;
    let mut embed_q: Vec<SplitResult> = vec![];

    loop {
        let (doc_mb, files_unprocessed) = {
            let mut queue_locked = queue.lock().await;
            let q_len =  queue_locked.len();
            (queue_locked.pop_front(), q_len)
        };

        loop {
            if embed_q.len() >= B || (!embed_q.is_empty() && files_unprocessed == 0) {
                vectorize_batch_from_q(&mut embed_q, status.clone(), client.clone(), &constants, &api_key, vecdb_handler_ref.clone(), B).await.unwrap_or_else(|err| {
                    warn!("Error vectorizing: {}", err);
                });
            } else {
                break;
            }
        }

        if (files_unprocessed + 99).div(100) != (reported_unprocessed + 99).div(100) {
            info!("have {} unprocessed files", files_unprocessed);
            reported_unprocessed = files_unprocessed;
        }

        reported_vecdb_complete &= files_unprocessed == 0;

        let mut doc = {
            match doc_mb {
                Some(doc) => {
                    let mut locked_status = status.lock().await;
                    locked_status.files_unprocessed = files_unprocessed;
                    if files_unprocessed > files_total {
                        files_total = files_unprocessed;
                    }
                    locked_status.files_total = files_total;
                    locked_status.state = "parsing".to_string();
                    doc
                },
                None => {
                    // No files left to process
                    if !reported_vecdb_complete {
                        let t0 = std::time::Instant::now();
                        vecdb_handler_ref.lock().await.update_indexed_file_paths().await;
                        info!("update_indexed_file_paths: it took {:.3}s", t0.elapsed().as_secs_f64());
                        reported_vecdb_complete = true;
                        // For now, we do not create index 'cause it hurts quality of retrieval
                        // info!("VECDB Creating index");
                        // match vecdb_handler_ref.lock().await.create_index().await {
                        //     Ok(_) => info!("VECDB CREATED INDEX"),
                        //     Err(err) => info!("VECDB Error creating index: {}", err)
                        // }
                        let _ = write!(std::io::stderr(), "VECDB COMPLETE\n");
                        info!("VECDB COMPLETE"); // you can see stderr "VECDB COMPLETE" sometimes faster vs logs
                        {
                            let mut locked_status = status.lock().await;
                            locked_status.files_unprocessed = 0;
                            locked_status.files_total = 0;
                            locked_status.state = "done".to_string();
                            info!(
                                "vectorizer since start {} API calls, {} vectors",
                                locked_status.requests_made_since_start, locked_status.vectors_made_since_start
                            );
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                    continue;
                }
            }
        };
        let last_30_chars = crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30);

        // Not from memory, vecdb works on files from disk
        if let Err(err) = doc.update_text_from_disk().await {
            info!("{}: {}", last_30_chars, err);
            continue;
        }

        if let Err(err) = doc.does_text_look_good() {
            info!("embeddings {} doesn't look good: {}", last_30_chars, err);
            continue;
        }

        let file_splitter = AstBasedFileSplitter::new();
        let split_data = file_splitter.vectorization_split(&doc, tokenizer.clone(), gcx_weak.clone(), constants.vectorizer_n_ctx).await.unwrap_or_else(|err| {
            info!("{}", err);
            vec![]
        });

        let file_split_data = {
            let mut vecdb_handler = vecdb_handler_ref.lock().await;
            let res = split_data
                .iter()
                .filter(|x| !vecdb_handler.contains(&x.window_text_hash))
                .cloned() // Clone to avoid borrowing issues
                .collect();
            vecdb_handler.try_add_from_cache(res).await
        };
        embed_q.extend(file_split_data);
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
                files_unprocessed: 0,
                files_total: 0,
                requests_made_since_start: 0,
                vectors_made_since_start: 0,
                db_size: 0,
                db_cache_size: 0,
                state: "starting".to_string(),
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

    pub async fn vecdb_start_background_tasks(
        &self,
        vecdb_client: Arc<AMutex<reqwest::Client>>,
        gcx: Arc<RwLock<GlobalContext>>,
        tokenizer: Arc<StdRwLock<Tokenizer>>,
    ) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                self.status.clone(),
                self.constants.cooldown_secs,
            )
        );

        let constants = self.constants.clone();
        let retrieve_thread_handle = tokio::spawn(
            vectorize_thread(
                vecdb_client.clone(),
                self.output_queue.clone(),
                self.vecdb_handler.clone(),
                self.status.clone(),
                constants,
                self.api_key.clone(),
                tokenizer,
                Arc::downgrade(&gcx.clone())
            )
        );

        let cleanup_thread_handle = tokio::spawn(
            cleanup_thread(
                self.vecdb_handler.clone()
            )
        );

        return vec![cooldown_queue_join_handle, retrieve_thread_handle, cleanup_thread_handle];
    }

    pub async fn vectorizer_enqueue_files(&self, documents: &Vec<Document>, force: bool) {
        info!("adding {} files", documents.len());
        if !force {
            self.update_request_queue.lock().await.extend(documents.clone());
        } else {
            self.output_queue.lock().await.extend(documents.clone());
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
