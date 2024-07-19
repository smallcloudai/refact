use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::ops::Div;
use std::sync::{Arc, Weak};
use std::sync::RwLock as StdRwLock;
use std::time::SystemTime;

use tokenizers::Tokenizer;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock, Notify as ANotify};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{info, warn};

use crate::ast::file_splitter::AstBasedFileSplitter;
use crate::fetch_embedding::get_embedding_with_retry;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::knowledge::{MemoriesDatabase, vectorize_dirty_memories};
use crate::vecdb::vdb_lance::VecDBHandler;
use crate::vecdb::vdb_structs::{VecdbRecord, SplitResult, VecdbConstants, VecDbStatus, SimpleTextHashVector};
use crate::vecdb::vdb_cache::VecDBCache;

const DEBUG_WRITE_VECDB_FILES: bool = false;


enum MessageToVecdbThread {
    RegularDocument(Document),
    MemoriesSomethingDirty(),
}

pub struct FileVectorizerService {
    vecdb_delayed_q: Arc<AMutex<VecDeque<Document>>>,
    vecdb_immediate_q: Arc<AMutex<VecDeque<MessageToVecdbThread>>>,
    pub vecdb_handler: Arc<AMutex<VecDBHandler>>,
    pub vecdb_cache: Arc<AMutex<VecDBCache>>,
    pub vstatus: Arc<AMutex<VecDbStatus>>,
    pub vstatus_notify: Arc<ANotify>,   // fun stuff https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
    constants: VecdbConstants,
    api_key: String,
    memdb: Arc<AMutex<MemoriesDatabase>>,
}

async fn cooldown_queue_thread(
    vecdb_delayed_q: Arc<AMutex<VecDeque<Document>>>,
    vecdb_immediate_q: Arc<AMutex<VecDeque<MessageToVecdbThread>>>,
    _vstatus: Arc<AMutex<VecDbStatus>>,
    cooldown_secs: u64,
) {
    // This function delays vectorization of a file, until mtime is at least cooldown_secs old.
    let mut last_updated: HashMap<Document, SystemTime> = HashMap::new();
    loop {
        let mut docs: Vec<Document> = Vec::new();
        {
            let mut queue_locked = vecdb_delayed_q.lock().await;
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
            vecdb_immediate_q.lock().await.push_back(MessageToVecdbThread::RegularDocument(doc));
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn vectorize_batch_from_q(
    embed_q: &mut Vec<SplitResult>,
    vstatus: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<reqwest::Client>>,
    constants: &VecdbConstants,
    api_key: &String,
    vecdb_handler_arc: Arc<AMutex<VecDBHandler>>,
    vecdb_cache_arc: Arc<AMutex<VecDBCache>>,
    #[allow(non_snake_case)]
    B: usize,
) -> Result<(), String> {
    let batch = embed_q.drain(..B.min(embed_q.len())).collect::<Vec<_>>();
    assert!(batch.len() > 0);
    let t0 = Instant::now();

    let batch_result = get_embedding_with_retry(
        client.clone(),
        &constants.endpoint_embeddings_style.clone(),
        &constants.model_name.clone(),
        &constants.endpoint_embeddings_template.clone(),
        batch.iter().map(|x| x.window_text.clone()).collect(),
        api_key,
        1,
    ).await?;

    if batch_result.len() != batch.len() {
        return Err(format!("vectorize: batch_result.len() != batch.len(): {} vs {}", batch_result.len(), batch.len()));
    }

    {
        let mut vstatus_locked = vstatus.lock().await;
        vstatus_locked.requests_made_since_start += 1;
        vstatus_locked.vectors_made_since_start += batch_result.len();
    }

    let mut vecdb_records = vec![];
    let mut cache_records = vec![];
    for (i, data_res) in batch.iter().enumerate() {
        if batch_result[i].is_empty() {
            info!("skipping an empty embedding split");
            continue;
        }
        vecdb_records.push(
            VecdbRecord {
                vector: Some(batch_result[i].clone()),
                window_text: data_res.window_text.clone(),
                window_text_hash: data_res.window_text_hash.clone(),
                file_path: data_res.file_path.clone(),
                start_line: data_res.start_line,
                end_line: data_res.end_line,
                distance: -1.0,
                usefulness: 0.0,
            }
        );
        cache_records.push(
            SimpleTextHashVector {
                vector: Some(batch_result[i].clone()),
                window_text: data_res.window_text.clone(),
                window_text_hash: data_res.window_text_hash.clone(),
            }
        );
    }

    if vecdb_records.len() > 0 {
        info!("embeddings got {} records in {}ms", vecdb_records.len(), t0.elapsed().as_millis());
        match vecdb_handler_arc.lock().await.add_or_update(&vecdb_records).await {
            Err(e) => {
                warn!("Error adding/updating records in VecDB: {}", e);
            }
            _ => {}
        }
    }

    if cache_records.len() > 0 {
        match vecdb_cache_arc.lock().await.cache_add_new_records(cache_records).await {
            Err(e) => {
                warn!("Error adding records to the cacheDB: {}", e);
            }
            _ => {}
        }
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;  // be nice to the server: up to 60 requests per minute

    Ok(())
}

async fn add_from_cache_to_vec_db(
    delayed_cached_splits_q: &mut Vec<SplitResult>,
    vecdb_handler_arc: Arc<AMutex<VecDBHandler>>,
    vecdb_cache_arc: Arc<AMutex<VecDBCache>>,
    group_size: usize,
) {
    info!("add_from_cache_to_vec_db: {} delayed cached splits in queue", delayed_cached_splits_q.len());
    while !delayed_cached_splits_q.is_empty() {
        let batch = delayed_cached_splits_q
            .drain(..group_size.min(delayed_cached_splits_q.len()))
            .collect::<Vec<_>>();
        let t0 = std::time::Instant::now();
        let records: Vec<VecdbRecord> = match vecdb_cache_arc.lock().await.get_records_by_splits(&batch).await {
            Ok((records, non_found_splits)) => {
                assert!(non_found_splits.is_empty());
                records
            }
            Err(err) => {
                info!("Error getting records from cache: {}", err);
                vec![]
            }
        };
        info!("read {} delayed cached splits from cache db took {:.3}s", batch.len(), t0.elapsed().as_secs_f32());
        match vecdb_handler_arc.lock().await.add_or_update(&records).await {
            Err(e) => {
                warn!("Error adding/updating records in VecDB: {}", e);
            }
            _ => {}
        }
    }
    info!("add_from_cache_to_vec_db: done");
}

async fn vectorize_thread(
    client: Arc<AMutex<reqwest::Client>>,
    vecdb_immediate_q: Arc<AMutex<VecDeque<MessageToVecdbThread>>>,
    vecdb_handler_arc: Arc<AMutex<VecDBHandler>>,
    vecdb_cache_arc: Arc<AMutex<VecDBCache>>,
    memdb: Arc<AMutex<MemoriesDatabase>>,
    vstatus: Arc<AMutex<VecDbStatus>>,
    vstatus_notify: Arc<ANotify>,
    constants: VecdbConstants,
    api_key: String,
    tokenizer: Arc<StdRwLock<Tokenizer>>,
    gcx_weak: Weak<ARwLock<GlobalContext>>,
) {
    const B: usize = 64;

    let mut files_total: usize = 0;
    let mut reported_unprocessed: usize = 0;
    let mut embed_q: Vec<SplitResult> = vec![];
    let mut delayed_cached_splits_q: Vec<SplitResult> = vec![];

    loop {
        let (msg_to_me, files_unprocessed, vstatus_changed) = {
            let mut qlocked = vecdb_immediate_q.lock().await;
            let q_len = qlocked.len();
            // CAREFUL: two locks, qlocked -> vstatus_locked
            let mut state_changed = false;
            {
                let mut vstatus_locked = vstatus.lock().await;
                if q_len == 0 {
                    if vstatus_locked.queue_additions {
                        vstatus_locked.queue_additions = false;
                        state_changed = true;
                    }
                    // will set "done" but later
                } else {
                    if vstatus_locked.state != "parsing" {
                        vstatus_locked.state = "parsing".to_string();
                        state_changed = true;
                    }
                }
            }
            (qlocked.pop_front(), q_len, state_changed)
        };
        if vstatus_changed {
            vstatus_notify.notify_waiters();
        }

        loop {
            if embed_q.len() >= B || (!embed_q.is_empty() && files_unprocessed == 0) {
                vectorize_batch_from_q(
                    &mut embed_q,
                    vstatus.clone(),
                    client.clone(),
                    &constants,
                    &api_key,
                    vecdb_handler_arc.clone(),
                    vecdb_cache_arc.clone(),
                    B,
                ).await.unwrap_or_else(|err| {
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

        let mut doc = {
            match msg_to_me {
                Some(MessageToVecdbThread::RegularDocument(doc)) => {
                    {
                        let mut vstatus_locked = vstatus.lock().await;
                        vstatus_locked.files_unprocessed = files_unprocessed;
                        if files_unprocessed > files_total {
                            files_total = files_unprocessed;
                        }
                        vstatus_locked.files_total = files_total;
                    }
                    vstatus_notify.notify_waiters();
                    doc
                }
                Some(MessageToVecdbThread::MemoriesSomethingDirty()) => {
                    info!("MEMDB VECTORIZER START");
                    let r = vectorize_dirty_memories(
                        memdb.clone(),
                        vecdb_cache_arc.clone(),
                        vstatus.clone(),
                        client.clone(),
                        &api_key,
                        B,
                    ).await;
                    info!("/MEMDB {:?}", r);
                    continue;
                }
                None => {
                    // No files left to process
                    if delayed_cached_splits_q.len() > 0 {
                        add_from_cache_to_vec_db( // leftover splits
                            &mut delayed_cached_splits_q,
                            vecdb_handler_arc.clone(),
                            vecdb_cache_arc.clone(),
                            1024,
                        ).await;
                    }
                    let reported_vecdb_complete = {
                        let mut vstatus_locked = vstatus.lock().await;
                        let done = vstatus_locked.state == "done";
                        if !done {
                            vstatus_locked.files_unprocessed = 0;
                            vstatus_locked.files_total = 0;
                            vstatus_locked.state = "done".to_string();
                            info!(
                                "vectorizer since start {} API calls, {} vectors",
                                vstatus_locked.requests_made_since_start, vstatus_locked.vectors_made_since_start
                            );
                        }
                        done
                    };
                    if !reported_vecdb_complete {
                        // For now, we do not create index because it hurts the quality of retrieval
                        // info!("VECDB Creating index");
                        // match vecdb_handler_arc.lock().await.create_index().await {
                        //     Ok(_) => info!("VECDB CREATED INDEX"),
                        //     Err(err) => info!("VECDB Error creating index: {}", err)
                        // }
                        let _ = write!(std::io::stderr(), "VECDB COMPLETE\n");
                        info!("VECDB COMPLETE"); // you can see stderr "VECDB COMPLETE" sometimes faster vs logs
                        vstatus_notify.notify_waiters();
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(5000)) => {},
                        _ = vstatus_notify.notified() => {},
                    }
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

        let file_splitter = AstBasedFileSplitter::new(constants.splitter_window_size);
        let split_data = file_splitter.vectorization_split(&doc, tokenizer.clone(), gcx_weak.clone(), constants.vectorizer_n_ctx).await.unwrap_or_else(|err| {
            info!("{}", err);
            vec![]
        });

        if DEBUG_WRITE_VECDB_FILES {
            let path_vecdb = doc.path.with_extension("vecdb");
            if let Ok(mut file) = std::fs::File::create(path_vecdb) {
                let mut writer = std::io::BufWriter::new(&mut file);
                for chunk in split_data.iter() {
                    let beautiful_line = format!("\n\n------- {:?} {}-{} ------\n", chunk.symbol_path, chunk.start_line, chunk.end_line);
                    let _ = writer.write_all(beautiful_line.as_bytes());
                    let _ = writer.write_all(chunk.window_text.as_bytes());
                    let _ = writer.write_all(b"\n");
                }
            }
        }

        {
            let vecdb_cache = vecdb_cache_arc.lock().await;
            for split_item in split_data.into_iter() {
                if vecdb_cache.contains(&split_item.window_text_hash) {
                    delayed_cached_splits_q.push(split_item);
                } else {
                    embed_q.push(split_item);
                }
            }
        }
        // Do not keep too many
        if delayed_cached_splits_q.len() > 1024 {
            add_from_cache_to_vec_db(
                &mut delayed_cached_splits_q,
                vecdb_handler_arc.clone(),
                vecdb_cache_arc.clone(),
                1024,
            ).await;
        }
    }
}


impl FileVectorizerService {
    pub async fn new(
        vecdb_handler: Arc<AMutex<VecDBHandler>>,
        vecdb_cache: Arc<AMutex<VecDBCache>>,
        constants: VecdbConstants,
        api_key: String,
        memdb: Arc<AMutex<MemoriesDatabase>>,
    ) -> Self {
        let vecdb_delayed_q = Arc::new(AMutex::new(VecDeque::new()));
        let vecdb_immediate_q = Arc::new(AMutex::new(VecDeque::new()));
        let vstatus = Arc::new(AMutex::new(
            VecDbStatus {
                files_unprocessed: 0,
                files_total: 0,
                requests_made_since_start: 0,
                vectors_made_since_start: 0,
                db_size: 0,
                db_cache_size: 0,
                state: "starting".to_string(),
                queue_additions: true,
                vecdb_max_files_hit: false,
            }
        ));
        FileVectorizerService {
            vecdb_delayed_q: vecdb_delayed_q.clone(),
            vecdb_immediate_q: vecdb_immediate_q.clone(),
            vecdb_handler: vecdb_handler.clone(),
            vecdb_cache: vecdb_cache.clone(),
            vstatus: vstatus.clone(),
            vstatus_notify: Arc::new(ANotify::new()),
            constants,
            api_key,
            memdb,
        }
    }

    pub async fn vecdb_start_background_tasks(
        &self,
        vecdb_client: Arc<AMutex<reqwest::Client>>,
        gcx: Arc<ARwLock<GlobalContext>>,
        tokenizer: Arc<StdRwLock<Tokenizer>>,
    ) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.vecdb_delayed_q.clone(),
                self.vecdb_immediate_q.clone(),
                self.vstatus.clone(),
                self.constants.cooldown_secs,
            )
        );

        let constants = self.constants.clone();
        let retrieve_thread_handle = tokio::spawn(
            vectorize_thread(
                vecdb_client.clone(),
                self.vecdb_immediate_q.clone(),
                self.vecdb_handler.clone(),
                self.vecdb_cache.clone(),
                self.memdb.clone(),
                self.vstatus.clone(),
                self.vstatus_notify.clone(),
                constants,
                self.api_key.clone(),
                tokenizer,
                Arc::downgrade(&gcx.clone()),
            )
        );
        return vec![cooldown_queue_join_handle, retrieve_thread_handle];
    }
}


pub async fn vectorizer_enqueue_dirty_memory(
    vservice: Arc<AMutex<FileVectorizerService>>
) {
    let (immediate_q, vstatus, vstatus_notify) = {
        let service = vservice.lock().await;
        (
            service.vecdb_immediate_q.clone(),
            service.vstatus.clone(),
            service.vstatus_notify.clone(),
        )
    };
    {
        // CAREFUL: two locks, qlocked -> vstatus_locked
        let mut qlocked = immediate_q.lock().await;
        qlocked.push_back(MessageToVecdbThread::MemoriesSomethingDirty());
        vstatus.lock().await.queue_additions = true;
    }
    vstatus_notify.notify_waiters();
}

pub async fn vectorizer_enqueue_files(
    vservice: Arc<AMutex<FileVectorizerService>>,
    documents: &Vec<Document>,
    process_immediately: bool
) {
    info!("adding {} files", documents.len());
    let (delayed_q, immediate_q, vstatus, vstatus_notify, vecdb_max_files) = {
        let service = vservice.lock().await;
        (
            service.vecdb_delayed_q.clone(),
            service.vecdb_immediate_q.clone(),
            service.vstatus.clone(),
            service.vstatus_notify.clone(),
            service.constants.vecdb_max_files
        )
    };
    let mut documents_my_copy = documents.clone();
    if documents_my_copy.len() > vecdb_max_files {
        info!("that's more than {} allowed in the command line, reduce the number", vecdb_max_files);
        documents_my_copy.truncate(vecdb_max_files);
        vstatus.lock().await.vecdb_max_files_hit = true;
    }
    if !process_immediately {
        delayed_q.lock().await.extend(documents.clone());
    } else {
        {
            // CAREFUL: two locks, qlocked -> vstatus_locked
            let mut qlocked = immediate_q.lock().await;
            for doc in documents.iter() {
                qlocked.push_back(MessageToVecdbThread::RegularDocument(doc.clone()));
            }
            vstatus.lock().await.queue_additions = true;
        }
        vstatus_notify.notify_waiters();
    }
}
