use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::task::JoinHandle;
use async_trait::async_trait;
use tracing::{error, info};
use serde_json;

use crate::background_tasks::BackgroundTasksHolder;
use crate::fetch_embedding;
use crate::global_context::{CommandLine, GlobalContext};
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{MemoRecord, MemoSearchResult, SearchResult, VecDbStatus, VecdbConstants, VecdbSearch};
use crate::vecdb::vdb_thread::{vecdb_start_background_tasks, vectorizer_enqueue_files, FileVectorizerService};


pub struct VecDb {
    vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    pub vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    // cmdline: CommandLine,  // TODO: take from command line what's needed, don't store a copy
    constants: VecdbConstants,
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
        VecdbConstants {
            embedding_model: caps.embedding_model.clone(),
            tokenizer: None,
            splitter_window_size: caps.embedding_model.base.n_ctx / 2,
            vecdb_max_files: vecdb_max_files,
        }
    };

    let vec_db = gcx.write().await.vec_db.clone();
    match *vec_db.lock().await {
        None => {}
        Some(ref db) => {
            if
                db.constants.embedding_model == consts.embedding_model &&
                db.constants.splitter_window_size == consts.splitter_window_size
            {
                return (false, None);
            }
        }
    }

    if consts.embedding_model.base.name.is_empty() || consts.embedding_model.base.endpoint.is_empty() {
        error!("command line says to launch vecdb, but this will not happen: embedding model name or endpoint are empty");
        return (true, None);
    }

    let tokenizer_result = crate::tokens::cached_tokenizer(
        gcx.clone(), &consts.embedding_model.base,
    ).await;
    
    consts.tokenizer = match tokenizer_result {
        Ok(tokenizer) => tokenizer,
        Err(err) => {
            error!("vecdb launch failed, embedding model tokenizer didn't load: {}", err);
            return (false, None);
        }
    };
    return (true, Some(consts));
}

pub async fn vecdb_background_reload(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let cmd_line = gcx.read().await.cmdline.clone();
    if !cmd_line.vecdb {
        return;
    }

    let mut background_tasks = BackgroundTasksHolder::new(vec![]);
    loop {
        let (need_reload, consts) = do_i_need_to_reload_vecdb(gcx.clone()).await;
        if need_reload {
            background_tasks.abort().await;
        }
        if need_reload && consts.is_some() {
            background_tasks = BackgroundTasksHolder::new(vec![]);
            
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
                Ok(_) => {
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
        let handler = VecDBSqlite::init(cache_dir, &constants.embedding_model.base.name, constants.embedding_model.embedding_size, &emb_table_name).await?;
        let vecdb_handler = Arc::new(AMutex::new(handler));
        let vectorizer_service = Arc::new(AMutex::new(FileVectorizerService::new(
            vecdb_handler.clone(),
            constants.clone(),
        ).await));
        let mut http_client_builder = reqwest::Client::builder();
        if cmdline.insecure {
            http_client_builder = http_client_builder.danger_accept_invalid_certs(true)
        }
        let vecdb_emb_client = Arc::new(AMutex::new(http_client_builder.build().unwrap()));
        Ok(VecDb {
            vecdb_emb_client,
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
        vecdb_start_background_tasks(self.vecdb_emb_client.clone(), self.vectorizer_service.clone(), gcx.clone()).await
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
    gcx: Arc<ARwLock<GlobalContext>>,
    project_name: &str,
    m_type: &str,
    m_memory: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let body = serde_json::json!({
        "project_name": project_name,
        "knowledge_type": m_type,
        "knowledge_origin": "client",
        "knowledge_memory": m_memory
    });
    let response = client.post("https://test-teams-v1.smallcloud.ai/v1/knowledge/upload?workspace_id=1")
        .header("Authorization", format!("Bearer {}", "sk_acme_13579"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Successfully added memory to remote server");
                Ok(())
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to add memory: HTTP status {}, error: {}", status, error_text))
            }
        },
        Err(e) => Err(format!("Failed to send memory add request: {}", e))
    }
}


pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    project_name: &str,
    query: &String,
    top_n: usize,
) -> Result<MemoSearchResult, String> {
    let client = reqwest::Client::new();
    let api_key = gcx.read().await.cmdline.api_key.clone();
    let url = format!("https://test-teams-v1.smallcloud.ai/v1/vecdb-search?workspace_id=1&limit={}", top_n);
    
    let body = serde_json::json!({
        "project_name": project_name,
        "q": query
    });
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", "sk_acme_13579"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_body = resp.text().await.map_err(|e| format!("Failed to read response body: {}", e))?;
                let results: Vec<MemoRecord> = serde_json::from_str(&response_body)
                    .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
                Ok(MemoSearchResult {
                    query_text: query.clone(),
                    project_name: project_name.to_string(),
                    results,
                })
            } else {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to search memories: HTTP status {}, error: {}", status, error_text))
            }
        },
        Err(e) => Err(format!("Failed to send memory search request: {}", e))
    }
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
    Ok(Some(vstatus_copy))
}


#[async_trait]
impl VecdbSearch for VecDb {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
    ) -> Result<SearchResult, String> {
        // TODO: move out of struct, replace self with Arc
        let t0 = std::time::Instant::now();
        let embedding_mb = fetch_embedding::get_embedding_with_retries(
            self.vecdb_emb_client.clone(),
            &self.constants.embedding_model,
            vec![query.clone()],
            5,
        ).await;
        if embedding_mb.is_err() {
            return Err(embedding_mb.unwrap_err().to_string());
        }
        info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

        let mut handler_locked = self.vecdb_handler.lock().await;
        let t1 = std::time::Instant::now();
        let mut results = match handler_locked.vecdb_search(&embedding_mb.unwrap()[0], top_n, vecdb_scope_filter_mb).await {
            Ok(res) => res,
            Err(err) => { return Err(err.to_string()) }
        };
        info!("search itself {:.3}s", t1.elapsed().as_secs_f64());
        let mut dist0 = 0.0;
        let mut filtered_results = Vec::new();
        let rejection_threshold = self.constants.embedding_model.rejection_threshold;
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
