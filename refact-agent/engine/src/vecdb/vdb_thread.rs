use indexmap::IndexMap;
use std::collections::HashSet;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::ops::Div;
use std::option::Option;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{Mutex as AMutex, Notify as ANotify, RwLock as ARwLock};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::ast::file_splitter::AstBasedFileSplitter;
use crate::fetch_embedding::get_embedding_with_retry;
use crate::files_in_workspace::{is_path_to_enqueue_valid, Document};
use crate::global_context::GlobalContext;
use crate::knowledge::{vectorize_dirty_memories, MemoriesDatabase};
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{SimpleTextHashVector, SplitResult, VecDbStatus, VecdbConstants, VecdbRecord};

const DEBUG_WRITE_VECDB_FILES: bool = false;
const COOLDOWN_SECONDS: u64 = 10;


enum MessageToVecdbThread {
    RegularDocument(String),
    ImmediatelyRegularDocument(String),
    MemoriesSomethingDirty(),
}

pub struct FileVectorizerService {
    pub vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    pub vstatus: Arc<AMutex<VecDbStatus>>,
    pub vstatus_notify: Arc<ANotify>,   // fun stuff https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
    constants: VecdbConstants,
    api_key: String,
    memdb: Arc<AMutex<MemoriesDatabase>>,
    vecdb_todo: Arc<AMutex<VecDeque<MessageToVecdbThread>>>,
}

async fn vectorize_batch_from_q(
    run_actual_model_on_these: &mut Vec<SplitResult>,
    ready_to_vecdb: &mut Vec<VecdbRecord>,
    vstatus: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<reqwest::Client>>,
    constants: &VecdbConstants,
    api_key: &String,
    vecdb_handler_arc: Arc<AMutex<VecDBSqlite>>,
    #[allow(non_snake_case)]
    B: usize,
) -> Result<(), String> {
    let batch = run_actual_model_on_these.drain(..B.min(run_actual_model_on_these.len())).collect::<Vec<_>>();
    assert!(batch.len() > 0);

    let batch_result = match get_embedding_with_retry(
        client.clone(),
        &constants.endpoint_embeddings_style.clone(),
        &constants.embedding_model.clone(),
        &constants.endpoint_embeddings_template.clone(),
        batch.iter().map(|x| x.window_text.clone()).collect(),
        api_key,
        10,
    ).await {
        Ok(res) => res,
        Err(e) => {
            let mut vstatus_locked = vstatus.lock().await;
            vstatus_locked.vecdb_errors.entry(e.clone()).and_modify(|counter| *counter += 1).or_insert(1);
            return Err(e);
        }
    };

    if batch_result.len() != batch.len() {
        return Err(format!("vectorize: batch_result.len() != batch.len(): {} vs {}", batch_result.len(), batch.len()));
    }

    {
        let mut vstatus_locked = vstatus.lock().await;
        vstatus_locked.requests_made_since_start += 1;
        vstatus_locked.vectors_made_since_start += batch_result.len();
    }

    let mut send_to_cache = vec![];
    for (i, data_res) in batch.iter().enumerate() {
        if batch_result[i].is_empty() {
            info!("skipping an empty embedding split");
            continue;
        }
        ready_to_vecdb.push(
            VecdbRecord {
                vector: Some(batch_result[i].clone()),
                file_path: data_res.file_path.clone(),
                start_line: data_res.start_line,
                end_line: data_res.end_line,
                distance: -1.0,
                usefulness: 0.0,
            }
        );
        send_to_cache.push(
            SimpleTextHashVector {
                vector: Some(batch_result[i].clone()),
                window_text: data_res.window_text.clone(),
                window_text_hash: data_res.window_text_hash.clone(),
            }
        );
    }

    if send_to_cache.len() > 0 {
        match vecdb_handler_arc.lock().await.cache_add_new_records(send_to_cache).await {
            Err(e) => {
                warn!("Error adding records to the cacheDB: {}", e);
            }
            _ => {}
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;  // be nice to the server: up to 60 requests per minute

    Ok(())
}

async fn from_splits_to_vecdb_records_applying_cache(
    splits: &mut Vec<SplitResult>,
    ready_to_vecdb: &mut Vec<VecdbRecord>,
    run_actual_model_on_these: &mut Vec<SplitResult>,
    vecdb_handler_arc: Arc<AMutex<VecDBSqlite>>,
    group_size: usize,
) {
    while !splits.is_empty() {
        let batch: Vec<SplitResult> = splits
            .drain(..group_size.min(splits.len()))
            .collect::<Vec<_>>();
        // let t0 = std::time::Instant::now();
        let vectors_maybe = vecdb_handler_arc.lock().await.fetch_vectors_from_cache(&batch).await;
        if let Ok(vectors) = vectors_maybe {
            // info!("query cache {} -> {} records {:.3}s", batch.len(), vectors.len(), t0.elapsed().as_secs_f32());
            for (split, maybe_vector) in batch.iter().zip(vectors.iter()) {
                if maybe_vector.is_none() {
                    run_actual_model_on_these.push(split.clone());
                    continue;
                }
                ready_to_vecdb.push(VecdbRecord {
                    vector: maybe_vector.clone(),
                    file_path: split.file_path.clone(),
                    start_line: split.start_line,
                    end_line: split.end_line,
                    distance: -1.0,
                    usefulness: 0.0,
                });
            }
        } else if let Err(err) = vectors_maybe {
            tracing::error!("{}", err);
        }
    }
}

async fn vectorize_thread(
    client: Arc<AMutex<reqwest::Client>>,
    vservice: Arc<AMutex<FileVectorizerService>>,
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let mut files_total: usize = 0;
    let mut files_unprocessed: usize;
    let mut reported_unprocessed: usize = 0;
    let mut run_actual_model_on_these: Vec<SplitResult> = vec![];
    let mut ready_to_vecdb: Vec<VecdbRecord> = vec![];

    let (vecdb_todo,
        memdb,
        constants,
        vecdb_handler_arc,
        vstatus,
        vstatus_notify,
        api_key
    ) = {
        let vservice_locked = vservice.lock().await;
        (
            vservice_locked.vecdb_todo.clone(),
            vservice_locked.memdb.clone(),
            vservice_locked.constants.clone(),
            vservice_locked.vecdb_handler.clone(),
            vservice_locked.vstatus.clone(),
            vservice_locked.vstatus_notify.clone(),
            vservice_locked.api_key.clone()
        )
    };

    let mut last_updated: HashMap<String, SystemTime> = HashMap::new();
    loop {
        let mut work_on_one: Option<MessageToVecdbThread> = None;
        let current_time = SystemTime::now();
        let mut vstatus_changed = false;
        {
            let mut vecdb_todo_locked = vecdb_todo.lock().await;
            while let Some(msg) = vecdb_todo_locked.pop_front() {
                match msg {
                    MessageToVecdbThread::RegularDocument(cpath) => {
                        last_updated.insert(cpath, current_time);
                    }
                    MessageToVecdbThread::ImmediatelyRegularDocument(_) | MessageToVecdbThread::MemoriesSomethingDirty() => {
                        work_on_one = Some(msg);
                        break;
                    }
                }
            }
            if work_on_one.is_none() {
                let doc_to_remove = last_updated.iter()
                    .find(|(_, time)| time.elapsed().unwrap_or_default().as_secs() > COOLDOWN_SECONDS)
                    .map(|(doc, _)| doc.clone());

                if let Some(doc) = doc_to_remove {
                    work_on_one = Some(MessageToVecdbThread::RegularDocument(doc.clone()));
                    last_updated.remove(&doc);
                }
            }
            files_unprocessed = vecdb_todo_locked.len() + last_updated.len() + if work_on_one.is_some() { 1 } else { 0 };
            files_total = files_total.max(files_unprocessed);
            {
                // two locks in sequence, vecdb_todo.lock -> vstatus.lock
                let mut vstatus_locked = vstatus.lock().await;
                vstatus_locked.files_unprocessed = files_unprocessed;
                vstatus_locked.files_total = files_total;
                vstatus_locked.queue_additions = false;
                if work_on_one.is_some() && vstatus_locked.state != "parsing" {
                    vstatus_locked.state = "parsing".to_string();
                    vstatus_changed = true;
                }
                if work_on_one.is_none() && files_unprocessed > 0 && vstatus_locked.state != "cooldown" {
                    vstatus_locked.state = "cooldown".to_string();
                    vstatus_changed = true;
                }
            }
        }
        if vstatus_changed {
            vstatus_notify.notify_waiters();
        }

        let flush = ready_to_vecdb.len() > 100 || files_unprocessed == 0 || work_on_one.is_none();
        loop {
            if
            run_actual_model_on_these.len() > 0 && flush ||
                run_actual_model_on_these.len() >= constants.embedding_batch
            {
                if let Err(err) = vectorize_batch_from_q(
                    &mut run_actual_model_on_these,
                    &mut ready_to_vecdb,
                    vstatus.clone(),
                    client.clone(),
                    &constants,
                    &api_key,
                    vecdb_handler_arc.clone(),
                    constants.embedding_batch,
                ).await {
                    tracing::error!("{}", err);
                    continue;
                }
            } else {
                break;
            }
        }

        if flush {
            assert!(run_actual_model_on_these.len() == 0);
            // This function assumes it can delete records with the filenames mentioned, therefore assert above
            _send_to_vecdb(vecdb_handler_arc.clone(), &mut ready_to_vecdb).await;
        }

        if (files_unprocessed + 99).div(100) != (reported_unprocessed + 99).div(100) {
            info!("have {} unprocessed files", files_unprocessed);
            reported_unprocessed = files_unprocessed;
        }
        let cpath = {
            match work_on_one {
                Some(MessageToVecdbThread::RegularDocument(cpath)) |
                Some(MessageToVecdbThread::ImmediatelyRegularDocument(cpath)) => {
                    cpath.clone()
                }
                Some(MessageToVecdbThread::MemoriesSomethingDirty()) => {
                    info!("MEMDB VECTORIZER START");
                    let r = vectorize_dirty_memories(
                        memdb.clone(),
                        vecdb_handler_arc.clone(),
                        vstatus.clone(),
                        client.clone(),
                        &api_key,
                        constants.embedding_batch,
                    ).await;
                    info!("/MEMDB {:?}", r);
                    continue;
                }
                None if last_updated.is_empty() => {
                    // no more files
                    assert!(run_actual_model_on_these.is_empty());
                    assert!(ready_to_vecdb.is_empty());
                    let reported_vecdb_complete = {
                        let mut vstatus_locked = vstatus.lock().await;
                        let done = vstatus_locked.state == "done";
                        if !done {
                            files_total = 0;
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
                        {
                            let vstatus_locked = vstatus.lock().await;
                            if !vstatus_locked.vecdb_errors.is_empty() {
                                info!("VECDB ERRORS: {:#?}", vstatus_locked.vecdb_errors);
                            }
                        }
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(1_000)) => {},
                        _ = vstatus_notify.notified() => {},
                    }
                    continue;
                }
                _ => continue
            }
        };
        let last_30_chars = crate::nicer_logs::last_n_chars(&cpath, 30);

        // Not from memory, vecdb works on files from disk, because they change less
        let mut doc: Document = Document { doc_path: cpath.clone().into(), doc_text: None };
        if let Err(_) = doc.update_text_from_disk(gcx.clone()).await {
            info!("{} cannot read, deleting from index", last_30_chars);  // don't care what the error is, trivial (or privacy)
            match vecdb_handler_arc.lock().await.vecdb_records_remove(vec![doc.doc_path.to_string_lossy().to_string()]).await {
                Ok(_) => {}
                Err(err) => {
                    info!("VECDB Error removing: {}", err);                    
                }
            };
            continue;
        }

        if let Err(err) = doc.does_text_look_good() {
            info!("embeddings {} doesn't look good: {}", last_30_chars, err);
            continue;
        }

        let file_splitter = AstBasedFileSplitter::new(constants.splitter_window_size);
        let mut splits = file_splitter.vectorization_split(&doc, None, gcx.clone(), constants.vectorizer_n_ctx).await.unwrap_or_else(|err| {
            info!("{}", err);
            vec![]
        });

        // Adding the filename so it can also be searched
        if let Some(filename) = doc.doc_path.file_name().map(|f| f.to_string_lossy().to_string()) {
            splits.push(crate::vecdb::vdb_structs::SplitResult {
                file_path: doc.doc_path.clone(),
                window_text: filename.clone(),
                window_text_hash: crate::ast::chunk_utils::official_text_hashing_function(&filename),
                start_line: 0,
                end_line: if let Some(text) = doc.doc_text { text.lines().count() as u64 - 1 } else { 0 },
                symbol_path: "".to_string(),
            });
        }

        if DEBUG_WRITE_VECDB_FILES {
            let path_vecdb = doc.doc_path.with_extension("vecdb");
            if let Ok(mut file) = std::fs::File::create(path_vecdb) {
                let mut writer = std::io::BufWriter::new(&mut file);
                for chunk in splits.iter() {
                    let beautiful_line = format!("\n\n------- {:?} {}-{} -------\n", chunk.symbol_path, chunk.start_line, chunk.end_line);
                    let _ = writer.write_all(beautiful_line.as_bytes());
                    let _ = writer.write_all(chunk.window_text.as_bytes());
                    let _ = writer.write_all(b"\n");
                }
            }
        }

        from_splits_to_vecdb_records_applying_cache(
            &mut splits,
            &mut ready_to_vecdb,
            &mut run_actual_model_on_these,
            vecdb_handler_arc.clone(),
            1024,
        ).await;
    }
}

async fn _send_to_vecdb(
    vecdb_handler_arc: Arc<AMutex<VecDBSqlite>>,
    ready_to_vecdb: &mut Vec<VecdbRecord>,
) {
    while !ready_to_vecdb.is_empty() {
        let unique_file_paths: HashSet<String> = ready_to_vecdb.iter()
            .map(|x| x.file_path.to_str().unwrap_or("No filename").to_string())
            .collect();
        let unique_file_paths_vec: Vec<String> = unique_file_paths.into_iter().collect();
        match vecdb_handler_arc.lock().await.vecdb_records_remove(unique_file_paths_vec).await {
            Ok(_) => {}
            Err(err) => {
                info!("VECDB Error removing: {}", err);                                    
            }
        };
        let batch: Vec<VecdbRecord> = ready_to_vecdb.drain(..).collect();
        if !batch.is_empty() {
            match vecdb_handler_arc.lock().await.vecdb_records_add(&batch).await {
                Ok(_) => {}
                Err(err) => {
                    info!("VECDB Error adding: {}", err);                                                        
                }
            }
        }
    }
}

impl FileVectorizerService {
    pub async fn new(
        vecdb_handler: Arc<AMutex<VecDBSqlite>>,
        constants: VecdbConstants,
        api_key: String,
        memdb: Arc<AMutex<MemoriesDatabase>>,
    ) -> Self {
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
                vecdb_errors: IndexMap::new(),
            }
        ));
        FileVectorizerService {
            vecdb_handler: vecdb_handler.clone(),
            vstatus: vstatus.clone(),
            vstatus_notify: Arc::new(ANotify::new()),
            constants,
            api_key,
            memdb,
            vecdb_todo: Default::default(),
        }
    }
}

pub async fn vecdb_start_background_tasks(
    vecdb_client: Arc<AMutex<reqwest::Client>>,
    vservice: Arc<AMutex<FileVectorizerService>>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<JoinHandle<()>> {
    let retrieve_thread_handle = tokio::spawn(
        vectorize_thread(
            vecdb_client.clone(),
            vservice.clone(),
            gcx.clone(),
        )
    );
    vec![retrieve_thread_handle]
}

pub async fn vectorizer_enqueue_dirty_memory(
    vservice: Arc<AMutex<FileVectorizerService>>
) {
    let (vecdb_todo, vstatus, vstatus_notify) = {
        let service = vservice.lock().await;
        (
            service.vecdb_todo.clone(),
            service.vstatus.clone(),
            service.vstatus_notify.clone(),
        )
    };
    {
        // two locks in sequence, vecdb_todo.lock -> vstatus.lock
        let mut qlocked = vecdb_todo.lock().await;
        qlocked.push_back(MessageToVecdbThread::MemoriesSomethingDirty());
        vstatus.lock().await.queue_additions = true;
    }
    vstatus_notify.notify_waiters();
}

fn _filter_docs_to_enqueue(docs: &Vec<String>) -> Vec<String> {
    let mut rejected_reasons = HashMap::new();
    let mut filtered_docs = vec![];

    for d in docs {
        let path: std::path::PathBuf = d.clone().into();
        match is_path_to_enqueue_valid(&path) {
            Ok(_) => {
                filtered_docs.push(d.clone());
            }
            Err(e) => {
                rejected_reasons.entry(e.to_string()).and_modify(|x| *x += 1).or_insert(1);
            }
        }
    }
    if !rejected_reasons.is_empty() {
        info!("VecDB rejected docs to enqueue reasons:");
        for (reason, count) in &rejected_reasons {
            info!("    {:>6} {}", count, reason);
        }
    }
    filtered_docs
}

pub async fn vectorizer_enqueue_files(
    vservice: Arc<AMutex<FileVectorizerService>>,
    documents: &Vec<String>,
    process_immediately: bool,
) {
    info!("adding {} files", documents.len());
    let documents = _filter_docs_to_enqueue(documents);
    let (vecdb_todo, vstatus, vstatus_notify, vecdb_max_files) = {
        let service = vservice.lock().await;
        (
            service.vecdb_todo.clone(),
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
    {
        {
            // two locks in sequence, vecdb_todo.lock -> vstatus.lock
            let mut vecdb_todo_locked = vecdb_todo.lock().await;
            for doc in documents.iter() {
                if process_immediately {
                    vecdb_todo_locked.push_back(MessageToVecdbThread::ImmediatelyRegularDocument(doc.clone()));
                } else {
                    vecdb_todo_locked.push_back(MessageToVecdbThread::RegularDocument(doc.clone()));
                }
            }
            vstatus.lock().await.queue_additions = true;
        }
        if process_immediately {
            vstatus_notify.notify_waiters();
        }
    }
}
