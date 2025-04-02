use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;
use tracing::{error, info};
use crate::background_tasks::BackgroundTasksHolder;
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

pub struct VecDb {
    pub vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    pub vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    pub constants: VecdbConstants,
}

// TODO: should it be used for reloading while refact-lsp is running?
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

    let vec_db = gcx.read().await.vecdb.clone();
    match vec_db {
        None => {}
        Some(db) => {
            let vecdb_locked = db.lock().await;
            if
                vecdb_locked.constants.embedding_model == consts.embedding_model &&
                vecdb_locked.constants.endpoint_embeddings_template == consts.endpoint_embeddings_template &&
                vecdb_locked.constants.endpoint_embeddings_style == consts.endpoint_embeddings_style &&
                vecdb_locked.constants.splitter_window_size == consts.splitter_window_size &&
                vecdb_locked.constants.embedding_batch == consts.embedding_batch &&
                vecdb_locked.constants.embedding_size == consts.embedding_size
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
    (true, Some(consts))
}

pub async fn vecdb_background_reload(gcx: Arc<ARwLock<GlobalContext>>) {
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
            // Use the fail-safe initialization with retries
            let init_config = crate::vecdb::vdb_init::VecDbInitConfig {
                max_attempts: 5,
                initial_delay_ms: 10,
                max_delay_ms: 1000,
                backoff_factor: 2.0,
                test_search_after_init: true,
            };
            match crate::vecdb::vdb_init::initialize_vecdb_with_context(
                gcx.clone(),
                consts.unwrap(),
                Some(init_config),
            ).await {
                Ok(tasks) => {
                    background_tasks = tasks;
                    gcx.write().await.vec_db_error = "".to_string();
                    info!("vecdb: initialization successful");
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    gcx.write().await.vec_db_error = err_msg.clone();
                    error!("vecdb init failed: {}", err_msg);
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
        let vecdb_handler = Arc::new(AMutex::new(
            VecDBSqlite::init(cache_dir, &constants.embedding_model, constants.embedding_size, &emb_table_name).await?
        ));

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


pub async fn get_status(
    vectorizer_service: Arc<AMutex<FileVectorizerService>>
) -> Result<Option<VecDbStatus>, String> {
    let (vstatus, vecdb_handler) = {
        let vectorizer_locked = vectorizer_service.lock().await;
        (
            vectorizer_locked.vstatus.clone(),
            vectorizer_locked.vecdb.clone(),
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
        vectorizer_service: Option<Arc<AMutex<FileVectorizerService>>>,
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
        if let Some(vectorizer_service) = vectorizer_service {
            memories_block_until_vectorized(vectorizer_service.clone(), 5_000).await?;
        }
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
