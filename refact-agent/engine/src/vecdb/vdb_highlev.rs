use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use tracing::{error, info};
use indexmap::IndexMap;

use crate::background_tasks::BackgroundTasksHolder;
use crate::caps::get_custom_embedding_api_key;
use crate::fetch_embedding;
use crate::global_context::{CommandLine, GlobalContext};
use crate::trajectories::try_to_download_trajectories;
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{SearchResult, VecDbStatus, VecdbConstants, VecdbSearch};
use crate::vecdb::vectorizer_service::FileVectorizerService;


fn model_to_rejection_threshold(embedding_model: &str) -> f32 {
    match embedding_model {
        "text-embedding-3-small" => 0.63,
        "thenlper_gte" => 0.25,
        _ => 0.63,
    }
}

pub async fn memories_block_until_vectorized(
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


pub struct VecDb {
    pub vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    pub vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    pub constants: VecdbConstants,
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

    let (base_dir_cache, _) = match cmdline.vecdb_force_path.as_str() {
        "" => (cache_dir, config_dir.clone()),
        path => (PathBuf::from(path), PathBuf::from(path)),
    };
    
    // Get the memdb from global context - it should be initialized before this function is called
    let memdb = match gcx.read().await.memdb.clone() {
        Some(db) => db.clone(),
        None => {
            return Err("MemDb should be initialized before VecDb".to_string());
        }
    };
    
    // Step 2: Initialize VecDb
    let vec_db_mb = match VecDb::init(
        &base_dir_cache,
        cmdline.clone(),
        constants.clone(),
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

    // Step 3: Initialize FileVectorizerService
    info!("Initializing FileVectorizerService");
    use crate::vecdb::vectorizer_service::FileVectorizerService;
    let vectorizer_service = FileVectorizerService::new(
        vec_db.vecdb_handler.clone(),
        constants.clone(),
        api_key.clone(),
        memdb.clone(),
    ).await;

    // Store VecDb and FileVectorizerService in global context
    let vec_db_arc = Arc::new(AMutex::new(Some(vec_db)));
    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.vec_db = vec_db_arc.clone();
        gcx_locked.vectorizer_service = Arc::new(AMutex::new(Some(vectorizer_service.clone())));
    }
    
    // Step 4: Start vectorizer background tasks
    info!("Starting vectorizer background tasks");
    let vectorizer_service_arc = Arc::new(AMutex::new(vectorizer_service));
    use crate::vecdb::vectorizer_service::start_vectorizer_background_tasks;
    let tasks = start_vectorizer_background_tasks(
        vec_db_arc.lock().await.as_ref().unwrap().vecdb_emb_client.clone(),
        vectorizer_service_arc.clone(),
        gcx.clone()
    ).await;
    background_tasks.extend(tasks);
    
    // Enqueue files for vectorization
    crate::files_in_workspace::enqueue_all_files_from_workspace_folders(gcx.clone(), true, true).await;
    crate::files_in_jsonl::enqueue_all_docs_from_jsonl_but_read_first(gcx.clone(), true, true).await;

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
            
            // Initialize MemDb first if not already present
            let config_dir = gcx.read().await.config_dir.clone();
            let constants = consts.unwrap();
            let reset_memory = gcx.read().await.cmdline.reset_memory;
            
            if gcx.read().await.memdb.is_none() {
                info!("Initializing memdb in vecdb_background_reload");
                let memdb = crate::memdb::db_init::memdb_init(&config_dir, &constants, reset_memory).await;
                let mut gcx_locked = gcx.write().await;
                gcx_locked.memdb = Some(memdb.clone());
            }
            
            // Then initialize VecDb and FileVectorizerService
            match _create_vecdb(
                gcx.clone(),
                &mut background_tasks,
                constants,
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
        cmdline: CommandLine,
        constants: VecdbConstants,
    ) -> Result<VecDb, String> {
        let emb_table_name = crate::vecdb::vdb_emb_aux::create_emb_table_name(&vec![cmdline.workspace_folder]);
        let handler = VecDBSqlite::init(cache_dir, &constants.embedding_model, constants.embedding_size, &emb_table_name).await?;
        let vecdb_handler = Arc::new(AMutex::new(handler));

        let mut http_client_builder = reqwest::Client::builder();
        if cmdline.insecure {
            http_client_builder = http_client_builder.danger_accept_invalid_certs(true)
        }
        let vecdb_emb_client = Arc::new(AMutex::new(http_client_builder.build().unwrap()));

        Ok(VecDb {
            vecdb_emb_client,
            vecdb_handler,
            constants: constants.clone(),
        })
    }

    pub async fn remove_file(&self, file_path: &PathBuf) -> Result<(), String> {
        let mut handler_locked = self.vecdb_handler.lock().await;
        let file_path_str = file_path.to_string_lossy().to_string();
        handler_locked.vecdb_records_remove(vec![file_path_str]).await
    }
}

pub async fn get_status(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    vectorizer_service: Arc<AMutex<Option<FileVectorizerService>>>
) -> Result<Option<VecDbStatus>, String> {
    // Get the status from the vectorizer service if available
    if let Some(service) = vectorizer_service.lock().await.as_ref() {
        let vstatus = service.vstatus.lock().await.clone();
        return Ok(Some(vstatus));
    }
    
    // If vectorizer service is not available, get basic status from VecDb
    let vecdb_handler = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.vecdb_handler.clone()
    };
    
    // Create a placeholder status
    let mut vstatus_copy = VecDbStatus {
        files_unprocessed: 0,
        files_total: 0,
        requests_made_since_start: 0,
        vectors_made_since_start: 0,
        db_size: 0,
        db_cache_size: 0,
        state: "running".to_string(),
        queue_additions: false,
        vecdb_max_files_hit: false,
        vecdb_errors: IndexMap::new(),
    };
    vstatus_copy.db_size = match vecdb_handler.lock().await.size().await {
        Ok(res) => res,
        Err(err) => return Err(err)
    };
    vstatus_copy.db_cache_size = match vecdb_handler.lock().await.cache_size().await {
        Ok(res) => res,
        Err(err) => return Err(err.to_string())
    };
    
    Ok(Some(vstatus_copy))
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

        // Note: In the previous implementation, we would wait for vectorization to complete
        // but in the current architecture, vectorization is handled separately

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
