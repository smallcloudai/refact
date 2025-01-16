use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::task::JoinHandle;
use async_trait::async_trait;
use tracing::{error, info};

use crate::background_tasks::BackgroundTasksHolder;
use crate::caps::get_custom_embedding_api_key;
use crate::fetch_embedding;
use crate::global_context::{CommandLine, GlobalContext};
use crate::knowledge::{MemdbSubEvent, MemoriesDatabase};
use crate::trajectories::try_to_download_trajectories;
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{MemoRecord, MemoSearchResult, SearchResult, VecDbStatus, VecdbConstants, VecdbSearch};
use crate::vecdb::vdb_thread::{vecdb_start_background_tasks, vectorizer_enqueue_dirty_memory, vectorizer_enqueue_files, FileVectorizerService};


fn model_to_rejection_threshold(embedding_model: &str) -> f32 {
    match embedding_model {
        "text-embedding-3-small" => 0.63,
        "thenlper_gte" => 0.25,
        _ => 0.63,
    }
}


pub struct VecDb {
    pub memdb: Arc<AMutex<MemoriesDatabase>>,
    vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    pub vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    // cmdline: CommandLine,  // TODO: take from command line what's needed, don't store a copy
    constants: VecdbConstants,
}

async fn vecdb_test_request(
    vecdb: &VecDb,
    api_key: &String,
) -> Result<(), String> {
    let search_result = vecdb.vecdb_search("test query".to_string(), 3, None, api_key).await;
    match search_result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            error!("vecdb: test search failed: {}", e);
            Err("test search failed".to_string())
        }
    }
}

async fn _create_vecdb( 
    gcx: Arc<ARwLock<GlobalContext>>,
    background_tasks: &mut BackgroundTasksHolder,
    constants: VecdbConstants,
) -> Result<(), String> {
    info!("vecdb: attempting to launch");
    let api_key = get_custom_embedding_api_key(gcx.clone()).await;
    if let Err(err) = api_key {
        return Err(err.message);
    }

    let (cache_dir, config_dir, cmdline) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.config_dir.clone(), gcx_locked.cmdline.clone())
    };
    let api_key = api_key.unwrap();

    let (base_dir_cache, base_dir_config) = match cmdline.vecdb_force_path.as_str() {
        "" => (cache_dir, config_dir),
        path => (PathBuf::from(path), PathBuf::from(path)),
    };
    let vec_db_mb = match VecDb::init(
        &base_dir_cache,
        &base_dir_config,
        cmdline.clone(),
        constants,
        &api_key
    ).await {
        Ok(res) => Some(res),
        Err(err) => {
            error!("Ooops database is broken!
                Last error message: {}
                You can report this issue here:
                https://github.com/smallcloudai/refact-lsp/issues
                Also, you can run this to erase your db:
                `rm -rf ~/.cache/refact/refact_vecdb_cache`
                After that restart this LSP server or your IDE.", err);
            return Err(err);
        }
    };
    let vec_db = vec_db_mb.unwrap();

    match vecdb_test_request(&vec_db, &api_key).await {
        Ok(_) => {}
        Err(s) => { return Err(s); }
    }
    info!("vecdb: test request complete");

    // Enqueue files before background task starts: workspace files (needs vec_db in gcx)
    let vec_db_arc = Arc::new(AMutex::new(Some(vec_db)));
    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.vec_db = vec_db_arc.clone();
    }
    crate::files_in_workspace::enqueue_all_files_from_workspace_folders(gcx.clone(), true, true).await;
    crate::files_in_jsonl::enqueue_all_docs_from_jsonl_but_read_first(gcx.clone(), true, true).await;

    {
        let vec_db_locked = vec_db_arc.lock().await;
        let tasks = vec_db_locked.as_ref().unwrap().vecdb_start_background_tasks(gcx.clone()).await;
        background_tasks.extend(tasks);
    }

    Ok(())
}

async fn do_i_need_to_reload_vecdb(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> (bool, Option<VecdbConstants>) {
    let caps = match crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => caps,
        Err(e) => {
            // This branch makes caps error disappear, unless we print it right here:
            info!("vecdb: no caps, will not start or reload vecdb, the error was: {}", e);
            return (false, None);
        }
    };

    let vecdb_max_files = gcx.read().await.cmdline.vecdb_max_files;
    let mut consts = {
        let caps_locked = caps.read().unwrap();
        let mut b = caps_locked.embedding_batch;
        if b == 0 {
            b = 64;
        }
        if b > 256 {
            tracing::warn!("embedding_batch can't be higher than 256");
            b = 64;
        }
        VecdbConstants {
            embedding_model: caps_locked.embedding_model.clone(),
            embedding_size: caps_locked.embedding_size,
            embedding_batch: b,
            vectorizer_n_ctx: caps_locked.embedding_n_ctx,
            tokenizer: None,
            endpoint_embeddings_template: caps_locked.endpoint_embeddings_template.clone(),
            endpoint_embeddings_style: caps_locked.endpoint_embeddings_style.clone(),
            splitter_window_size: caps_locked.embedding_n_ctx / 2,
            vecdb_max_files: vecdb_max_files,
        }
    };

    let vec_db = gcx.write().await.vec_db.clone();
    match *vec_db.lock().await {
        None => {}
        Some(ref db) => {
            if
                db.constants.embedding_model == consts.embedding_model &&
                db.constants.endpoint_embeddings_template == consts.endpoint_embeddings_template &&
                db.constants.endpoint_embeddings_style == consts.endpoint_embeddings_style &&
                db.constants.splitter_window_size == consts.splitter_window_size &&
                db.constants.embedding_batch == consts.embedding_batch &&
                db.constants.embedding_size == consts.embedding_size
            {
                return (false, None);
            }
        }
    }

    if consts.embedding_model.is_empty() || consts.endpoint_embeddings_template.is_empty() {
        error!("command line says to launch vecdb, but this will not happen: embedding_model.is_empty() || endpoint_embeddings_template.is_empty()");
        return (true, None);
    }

    let tokenizer_maybe = crate::cached_tokenizers::cached_tokenizer(
        caps.clone(), gcx.clone(), consts.embedding_model.clone()).await;
    if tokenizer_maybe.is_err() {
        error!("vecdb launch failed, embedding model tokenizer didn't load: {}", tokenizer_maybe.unwrap_err());
        return (false, None);
    }
    consts.tokenizer = Some(tokenizer_maybe.clone().unwrap());

    return (true, Some(consts));
}

pub async fn vecdb_background_reload(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let cmd_line = gcx.read().await.cmdline.clone();
    if !cmd_line.vecdb {
        return;
    }

    let mut trajectories_updated_once: bool = false;
    let mut background_tasks = BackgroundTasksHolder::new(vec![]);
    loop {
        let (need_reload, consts) = do_i_need_to_reload_vecdb(gcx.clone()).await;
        if need_reload {
            background_tasks.abort().await;
        }
        if need_reload && consts.is_some() {
            background_tasks = BackgroundTasksHolder::new(vec![]);
            match _create_vecdb(
                gcx.clone(),
                &mut background_tasks,
                consts.unwrap(),
            ).await {
                Ok(_) => {
                    gcx.write().await.vec_db_error = "".to_string();
                }
                Err(err) => {
                    gcx.write().await.vec_db_error = err.clone();
                    error!("vecdb init failed: {}", err);
                    // gcx.vec_db stays None, the rest of the system continues working
                }
            }
        }
        if !trajectories_updated_once {
            match try_to_download_trajectories(gcx.clone()).await {
                Ok(_) => { }
                Err(err) => {
                    error!("trajectories download failed: {}", err);
                }
            };
            trajectories_updated_once = true;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

impl VecDb {
    pub async fn init(
        cache_dir: &PathBuf,
        config_dir: &PathBuf,
        cmdline: CommandLine,
        constants: VecdbConstants,
        api_key: &String
    ) -> Result<VecDb, String> {
        let handler = VecDBSqlite::init(cache_dir, &constants.embedding_model, constants.embedding_size).await?;
        let vecdb_handler = Arc::new(AMutex::new(handler));
        let memdb = Arc::new(AMutex::new(MemoriesDatabase::init(config_dir, &constants, cmdline.reset_memory).await?));

        let vectorizer_service = Arc::new(AMutex::new(FileVectorizerService::new(
            vecdb_handler.clone(),
            constants.clone(),
            api_key.clone(),
            memdb.clone(),
        ).await));
        Ok(VecDb {
            memdb: memdb.clone(),
            vecdb_emb_client: Arc::new(AMutex::new(reqwest::Client::new())),
            vecdb_handler,
            vectorizer_service,
            constants: constants.clone(),
        })
    }

    pub async fn vecdb_start_background_tasks(
        &self,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Vec<JoinHandle<()>> {
        info!("vecdb: start_background_tasks");
        vectorizer_enqueue_dirty_memory(self.vectorizer_service.clone()).await;
        return vecdb_start_background_tasks(self.vecdb_emb_client.clone(), self.vectorizer_service.clone(), gcx.clone()).await;
    }

    pub async fn vectorizer_enqueue_files(&self, documents: &Vec<String>, process_immediately: bool) {
        vectorizer_enqueue_files(self.vectorizer_service.clone(), documents, process_immediately).await;
    }

    pub async fn remove_file(&self, file_path: &PathBuf) -> Result<(), String> {
        let mut handler_locked = self.vecdb_handler.lock().await;
        let file_path_str = file_path.to_string_lossy().to_string();
        handler_locked.vecdb_records_remove(vec![file_path_str]).await
    }
}

pub async fn memories_add(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    m_type: &str,
    m_goal: &str,
    m_project: &str,
    m_payload: &str,    // TODO: upgrade to serde_json::Value
    m_origin: &str
) -> Result<String, String> {
    let (memdb, vectorizer_service) = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        (vec_db.memdb.clone(), vec_db.vectorizer_service.clone())
    };

    let memid = {
        let mut memdb_locked = memdb.lock().await;
        let x = memdb_locked.permdb_add(m_type, m_goal, m_project, m_payload, m_origin).await?;
        memdb_locked.dirty_memids.push(x.clone());
        x
    };
    vectorizer_enqueue_dirty_memory(vectorizer_service).await;  // sets queue_additions inside
    Ok(memid)
}

pub async fn memories_block_until_vectorized_from_vectorizer(
    vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    max_blocking_time_ms: usize
) -> Result<(), String> {
    let max_blocking_duration = tokio::time::Duration::from_millis(max_blocking_time_ms as u64);
    let start_time = std::time::Instant::now();
    let (vstatus, vstatus_notify) = {
        let service = vectorizer_service.lock().await;
        (service.vstatus.clone(), service.vstatus_notify.clone())
    };
    loop {
        let future: tokio::sync::futures::Notified = vstatus_notify.notified();
        {
            let vstatus_locked = vstatus.lock().await;
            if vstatus_locked.state == "done" && !vstatus_locked.queue_additions ||
                start_time.elapsed() >= max_blocking_duration
            {
                break;
            }
        }
        let remaining_time = max_blocking_duration
            .checked_sub(start_time.elapsed())
            .unwrap_or_else(|| tokio::time::Duration::from_millis(0));
        let sleep_duration = remaining_time
            .checked_add(tokio::time::Duration::from_millis(50))
            .unwrap_or_else(|| tokio::time::Duration::from_millis(50))
            .max(tokio::time::Duration::from_millis(1));
        tokio::select! {
            _ = future => {},
            _ = tokio::time::sleep(sleep_duration) => {},
        }
    };
    Ok(())
}

pub async fn memories_block_until_vectorized(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    max_blocking_time_ms: usize
) -> Result<(), String> {

    let vectorizer_service = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.vectorizer_service.clone()
    };
    memories_block_until_vectorized_from_vectorizer(vectorizer_service, max_blocking_time_ms).await
}

pub async fn get_status(vec_db: Arc<AMutex<Option<VecDb>>>) -> Result<Option<VecDbStatus>, String> {
    let vectorizer_service = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.vectorizer_service.clone()
    };
    let (vstatus, vecdb_handler) = {
        let vectorizer_locked = vectorizer_service.lock().await;
        (
            vectorizer_locked.vstatus.clone(),
            vectorizer_locked.vecdb_handler.clone(),
        )
    };
    let mut vstatus_copy = vstatus.lock().await.clone();
    vstatus_copy.db_size = match vecdb_handler.lock().await.size().await {
        Ok(res) => res,
        Err(err) => return Err(err)
    };
    vstatus_copy.db_cache_size = match vecdb_handler.lock().await.cache_size().await {
        Ok(res) => res,
        Err(err) => return Err(err.to_string())
    };
    if vstatus_copy.state == "done" && vstatus_copy.queue_additions {
        vstatus_copy.state = "cooldown".to_string();
    }
    return Ok(Some(vstatus_copy));
}

pub async fn memories_select_all(
    vec_db: Arc<AMutex<Option<VecDb>>>,
) -> Result<Vec<MemoRecord>, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let memdb_locked = memdb.lock().await;
    let results = memdb_locked.permdb_select_all().await?;
    Ok(results)
}

pub async fn memories_select_like(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    query: &String
) -> Result<Vec<MemoRecord>, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let memdb_locked = memdb.lock().await;
    let results = memdb_locked.permdb_select_like(query).await?;
    Ok(results)
}

pub async fn memories_erase(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    memid: &str,
) -> Result<usize, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let mut memdb_locked = memdb.lock().await;
    let erased_cnt = memdb_locked.permdb_erase(memid).await?;
    Ok(erased_cnt)
}

pub async fn memories_update(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    memid: &str,
    mstat_correct: i32,
    mstat_relevant: i32,
) -> Result<usize, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let memdb_locked = memdb.lock().await;
    let updated_cnt = memdb_locked.permdb_update_used(memid, mstat_correct, mstat_relevant).await?;
    Ok(updated_cnt)
}

pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &String,
    top_n: usize,
) -> Result<MemoSearchResult, String> {
    let vec_db = gcx.read().await.vec_db.clone();
    fn calculate_score(distance: f32, _times_used: i32) -> f32 {
        distance
        // distance - (times_used as f32) * 0.01
    }

    let t0 = std::time::Instant::now();
    let (memdb, vecdb_emb_client, constants) = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        (
            vec_db.memdb.clone(),
            vec_db.vecdb_emb_client.clone(),
            vec_db.constants.clone(),
        )
    };

    let api_key = get_custom_embedding_api_key(gcx.clone()).await;
    if let Err(err) = api_key {
        return Err(err.message);
    }

    let embedding = fetch_embedding::get_embedding_with_retry(
        vecdb_emb_client,
        &constants.endpoint_embeddings_style,
        &constants.embedding_model,
        &constants.endpoint_embeddings_template,
        vec![query.clone()],
        &api_key.unwrap(),
        5,
    ).await?;
    if embedding.is_empty() {
        return Err("memdb_search: empty embedding".to_string());
    }
    info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

    let mut results = {
        let memdb_locked = memdb.lock().await;
        memdb_locked.search_similar_records(&embedding[0], top_n).await?
    };
    results.sort_by(|a, b| {
        let score_a = calculate_score(a.distance, a.mstat_times_used);
        let score_b = calculate_score(b.distance, b.mstat_times_used);
        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(MemoSearchResult { query_text: query.clone(), results })
}

pub async fn memdb_subscription_poll(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    from_memid: Option<i64>
) -> Result<Vec<MemdbSubEvent>, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let x = memdb.lock().await.permdb_sub_select_all(from_memid).await; x
}


pub async fn memdb_pubsub_trigerred(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    sleep_seconds: u64
) -> Result<(), String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };
    let pubsub_notifier = memdb.lock().await.pubsub_notifier.clone();
    match tokio::time::timeout(tokio::time::Duration::from_secs(sleep_seconds), pubsub_notifier.notified()).await {
        Ok(_) => { },
        Err(_) => { }
    }
    Ok(())
}


#[async_trait]
impl VecdbSearch for VecDb {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
        api_key: &String,
    ) -> Result<SearchResult, String> {
        // TODO: move out of struct, replace self with Arc
        let t0 = std::time::Instant::now();
        let embedding_mb = fetch_embedding::get_embedding_with_retry(
            self.vecdb_emb_client.clone(),
            &self.constants.endpoint_embeddings_style,
            &self.constants.embedding_model,
            &self.constants.endpoint_embeddings_template,
            vec![query.clone()],
            api_key,
            5,
        ).await;
        if embedding_mb.is_err() {
            return Err(embedding_mb.unwrap_err().to_string());
        }
        info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

        memories_block_until_vectorized_from_vectorizer(self.vectorizer_service.clone(),
                                                        5_000).await?;

        let mut handler_locked = self.vecdb_handler.lock().await;
        let t1 = std::time::Instant::now();
        let mut results = match handler_locked.vecdb_search(&embedding_mb.unwrap()[0], top_n, vecdb_scope_filter_mb).await {
            Ok(res) => res,
            Err(err) => { return Err(err.to_string()) }
        };
        info!("search itself {:.3}s", t1.elapsed().as_secs_f64());
        let mut dist0 = 0.0;
        let mut filtered_results = Vec::new();
        let rejection_threshold = model_to_rejection_threshold(self.constants.embedding_model.as_str());
        info!("rejection_threshold {:.3}", rejection_threshold);
        for rec in results.iter_mut() {
            if dist0 == 0.0 {
                dist0 = rec.distance.abs();
            }
            let last_35_chars = crate::nicer_logs::last_n_chars(&rec.file_path.display().to_string(), 35);
            rec.usefulness = 100.0 - 75.0 * ((rec.distance.abs() - dist0) / (dist0 + 0.01)).max(0.0).min(1.0);
            if rec.distance.abs() >= rejection_threshold {
                info!("distance {:.3} -> dropped {}:{}-{}", rec.distance, last_35_chars, rec.start_line, rec.end_line);
            } else {
                info!("distance {:.3} -> useful {:.1}, found {}:{}-{}", rec.distance, rec.usefulness, last_35_chars, rec.start_line, rec.end_line);
                filtered_results.push(rec.clone());
            }
        }
        results = filtered_results;
        Ok(
            SearchResult {
                query_text: query,
                results,
            }
        )
    }
}
