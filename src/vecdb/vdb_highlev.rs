use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::sync::Mutex as StdMutex;
use std::collections::HashMap;
use tokenizers::Tokenizer;
use indexmap::IndexMap;
use tracing::{info, error};

use async_trait::async_trait;
use tokio::task::JoinHandle;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex };
use crate::global_context::{CommandLine, GlobalContext};
use crate::background_tasks::BackgroundTasksHolder;

use crate::fetch_embedding;
use crate::files_in_workspace::Document;
use crate::knowledge::{lance_search, MemoriesDatabase};
use crate::vecdb::vdb_lance::VecDBHandler;
use crate::vecdb::vdb_thread::{FileVectorizerService, vectorizer_enqueue_dirty_memory, vectorizer_enqueue_files};
use crate::vecdb::vdb_structs::{SearchResult, VecdbSearch, VecDbStatus, VecdbConstants, MemoRecord, MemoSearchResult, OngoingFlow};
use crate::vecdb::vdb_cache::VecDBCache;


fn vecdb_constants(
    caps: Arc<StdRwLock<crate::caps::CodeAssistantCaps>>,
    tokenizer: Arc<StdRwLock<Tokenizer>>,
) -> VecdbConstants {
    let caps_locked = caps.read().unwrap();
    VecdbConstants {
        model_name: caps_locked.default_embeddings_model.clone(),
        embedding_size: caps_locked.size_embeddings.clone(),
        vectorizer_n_ctx: caps_locked.embedding_n_ctx,
        tokenizer: tokenizer.clone(),
        endpoint_embeddings_template: caps_locked.endpoint_embeddings_template.clone(),
        endpoint_embeddings_style: caps_locked.endpoint_embeddings_style.clone(),
        cooldown_secs: 20,
        splitter_window_size: caps_locked.embedding_n_ctx / 2,
    }
}

pub struct VecDb {
    pub memdb: Arc<AMutex<MemoriesDatabase>>,
    vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    vecdb_handler: Arc<AMutex<VecDBHandler>>,
    vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    cmdline: CommandLine,  // TODO: take from command line what's needed, don't store a copy
    constants: VecdbConstants,
    pub mem_ongoing: Arc<StdMutex<HashMap<String, OngoingFlow>>>,
}

async fn vecdb_test_request(
    vecdb: &VecDb
) -> Result<(), String> {
    let search_result = vecdb.vecdb_search("test query".to_string(), 3, None).await;
    match search_result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            error!("vecdb: test search failed: {}", e);
            Err("vecdb: test search failed".to_string())
        }
    }
}

async fn _create_vecdb(
    gcx: Arc<ARwLock<GlobalContext>>,
    background_tasks: &mut BackgroundTasksHolder,
    constants: VecdbConstants,
) -> Result<(), String> {
    info!("vecdb: attempting to launch");

    let (cache_dir, cmdline, caps_arc_maybe) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.cmdline.clone(), gcx_locked.caps.clone())

    };
    if caps_arc_maybe.is_none() {
        return Err("no caps, will not start or reload vecdb".to_string());
    }

    let base_dir: PathBuf = match cmdline.vecdb_forced_path.as_str() {
        "" => cache_dir,
        path => PathBuf::from(path),
    };
    let vec_db_mb = match VecDb::init(
        &base_dir,
        cmdline.clone(),
        constants,
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

    match vecdb_test_request(&vec_db).await {
        Ok(_) => {},
        Err(s) => {return Err(s);}
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
            return (false, None)
        }
    };

    let model_name = {
        let caps_locked = caps.read().unwrap();
        caps_locked.default_embeddings_model.clone()
    };
    let tokenizer_maybe = crate::cached_tokenizers::cached_tokenizer(
        caps.clone(), gcx.clone(), model_name).await;
    if tokenizer_maybe.is_err() {
        error!("vecdb launch failed, embedding model tokenizer didn't load: {}", tokenizer_maybe.unwrap_err());
        return (false, None);
    }

    let consts = vecdb_constants(caps.clone(), tokenizer_maybe.unwrap());

    if consts.model_name.is_empty() || consts.endpoint_embeddings_template.is_empty() {
        error!("vecdb launch failed: default_embeddings_model.is_empty() || endpoint_embeddings_template.is_empty()");
        return (false, None);
    }

    match *gcx.write().await.vec_db.lock().await {
        None => {}
        Some(ref db) => {
            if db.constants.model_name == consts.model_name &&
                db.constants.endpoint_embeddings_template == consts.endpoint_embeddings_template &&
                db.constants.endpoint_embeddings_style == consts.endpoint_embeddings_style
            {
                return (false, None);
            }
        }
    }

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
        if need_reload && consts.is_some() {
            background_tasks.abort().await;
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
                    error!("vecdb: init failed: {}", err);
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
        let handler = VecDBHandler::init(constants.embedding_size).await?;
        let cache = VecDBCache::init(cache_dir, &constants.model_name, constants.embedding_size).await?;
        let vecdb_handler = Arc::new(AMutex::new(handler));
        let vecdb_cache = Arc::new(AMutex::new(cache));
        let memdb = Arc::new(AMutex::new(MemoriesDatabase::init(cache_dir, &constants, cmdline.reset_memory).await?));
        let vectorizer_service = Arc::new(AMutex::new(FileVectorizerService::new(
            vecdb_handler.clone(),
            vecdb_cache.clone(),
            constants.clone(),
            cmdline.api_key.clone(),
            memdb.clone(),
        ).await));
        Ok(VecDb {
            memdb: memdb.clone(),
            vecdb_emb_client: Arc::new(AMutex::new(reqwest::Client::new())),
            vecdb_handler,
            vectorizer_service,
            cmdline: cmdline.clone(),
            constants: constants.clone(),
            mem_ongoing: Arc::new(StdMutex::new(HashMap::<String, OngoingFlow>::new())),
        })
    }

    pub async fn vecdb_start_background_tasks(
        &self,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Vec<JoinHandle<()>> {
        info!("vecdb: start_background_tasks");
        vectorizer_enqueue_dirty_memory(self.vectorizer_service.clone()).await;
        return self.vectorizer_service.lock().await.vecdb_start_background_tasks(self.vecdb_emb_client.clone(), gcx.clone(), self.constants.tokenizer.clone()).await;
    }

    pub async fn vectorizer_enqueue_files(&self, documents: &Vec<Document>, process_immediately: bool) {
        vectorizer_enqueue_files(self.vectorizer_service.clone(), documents, process_immediately).await;
    }

    pub async fn remove_file(&self, file_path: &PathBuf) {
        self.vecdb_handler.lock().await.remove(file_path).await;
    }
}

pub async fn memories_add(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    m_type: &str,
    m_goal: &str,
    m_project: &str,
    m_payload: &str
) -> Result<String, String> {
    let (memdb, vectorizer_service) = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        (vec_db.memdb.clone(), vec_db.vectorizer_service.clone())
    };

    let memid = {
        let mut memdb_locked = memdb.lock().await;
        let x = memdb_locked.permdb_add(m_type, m_goal, m_project, m_payload)?;
        memdb_locked.dirty_memids.push(x.clone());
        x
    };
    vectorizer_enqueue_dirty_memory(vectorizer_service).await;  // sets queue_additions inside
    Ok(memid)
}

pub async fn memories_block_until_vectorized(
    vec_db: Arc<AMutex<Option<VecDb>>>,
) -> Result<(), String> {
    let vectorizer_service = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.vectorizer_service.clone()
    };
    let (vstatus, vstatus_notify) = {
        let service = vectorizer_service.lock().await;
        (service.vstatus.clone(), service.vstatus_notify.clone())
    };
    loop {
        let future: tokio::sync::futures::Notified = vstatus_notify.notified();
        {
            let vstatus_locked = vstatus.lock().await;
            if vstatus_locked.state == "done" && !vstatus_locked.queue_additions {
                break;
            }
        }
        future.await;
    };
    Ok(())
}

pub async fn get_status(vec_db: Arc<AMutex<Option<VecDb>>>) -> Result<Option<VecDbStatus>, String> {
    let vectorizer_service = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.vectorizer_service.clone()
    };
    let (vstatus, vecdb_handler, vecdb_cache) = {
        let vectorizer_locked = vectorizer_service.lock().await;
        (
            vectorizer_locked.vstatus.clone(),
            vectorizer_locked.vecdb_handler.clone(),
            vectorizer_locked.vecdb_cache.clone(),
        )
    };
    let mut vstatus_copy = vstatus.lock().await.clone();
    vstatus_copy.db_size = match vecdb_handler.lock().await.size().await {
        Ok(res) => res,
        Err(err) => return Err(err)
    };
    vstatus_copy.db_cache_size = match vecdb_cache.lock().await.size().await {
        Ok(res) => res,
        Err(err) => return Err(err.to_string())
    };
    if vstatus_copy.state == "done" && vstatus_copy.queue_additions {
        vstatus_copy.state = "parsing".to_string();
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
    let results = memdb_locked.permdb_select_all(None).await?;
    Ok(results)
}

pub async fn memories_erase(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    memid: &str
) -> Result<usize, String> {
    let memdb = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.memdb.clone()
    };

    let memdb_locked = memdb.lock().await;
    let erased_cnt = memdb_locked.permdb_erase(memid)?;
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
    let updated_cnt = memdb_locked.permdb_update_used(memid, mstat_correct, mstat_relevant)?;
    Ok(updated_cnt)
}

pub async fn memories_search(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    query: &String,
    top_n: usize,
) -> Result<MemoSearchResult, String> {
    fn calculate_score(distance: f32, _times_used: i32) -> f32 {
        distance
        // distance - (times_used as f32) * 0.01
    }

    let t0 = std::time::Instant::now();
    let (memdb, vecdb_emb_client, constants, cmdline) = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        (
            vec_db.memdb.clone(),
            vec_db.vecdb_emb_client.clone(),
            vec_db.constants.clone(),
            vec_db.cmdline.clone(),
        )
    };

    let embedding = fetch_embedding::get_embedding_with_retry(
        vecdb_emb_client,
        &constants.endpoint_embeddings_style,
        &constants.model_name,
        &constants.endpoint_embeddings_template,
        vec![query.clone()],
        &cmdline.api_key,
        5
    ).await?;
    if embedding.is_empty() {
        return Err("memdb_search: empty embedding".to_string());
    }
    info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

    let lance_results = match lance_search(memdb.clone(), &embedding[0], top_n).await {
        Ok(res) => res,
        Err(err) => { return Err(err.to_string()) }
    };
    let mut results: Vec<MemoRecord> = memdb.lock().await.permdb_fillout_records(lance_results).await?;
    results.sort_by(|a, b| {
        let score_a = calculate_score(a.distance, a.mstat_times_used);
        let score_b = calculate_score(b.distance, b.mstat_times_used);
        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(MemoSearchResult {query_text: query.clone(), results })
}

pub async fn ongoing_update_or_create(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    goal: String,
    ongoing_json: IndexMap<String, serde_json::Value>,
) -> Result<(), String> {
    let ongoing_map_arc = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.mem_ongoing.clone()
    };
    let mut ongoing_map = ongoing_map_arc.lock().unwrap();
    if let Some(ongoing) = ongoing_map.get_mut(&goal) {
        ongoing.ongoing_json = ongoing_json;
    } else {
        let new_ongoing = OngoingFlow {
            goal: goal.clone(),
            ongoing_json,
        };
        ongoing_map.insert(goal, new_ongoing);
    }
    Ok(())
}

pub async fn ongoing_find(
    vec_db: Arc<AMutex<Option<VecDb>>>,
    goal: String,
) -> Result<Option<OngoingFlow>, String> {
    let ongoing_map_arc = {
        let vec_db_guard = vec_db.lock().await;
        let vec_db = vec_db_guard.as_ref().ok_or("VecDb is not initialized")?;
        vec_db.mem_ongoing.clone()
    };
    let ongoing_map = ongoing_map_arc.lock().unwrap();
    Ok(ongoing_map.get(&goal).cloned())
}


#[async_trait]
impl VecdbSearch for VecDb {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
    ) -> Result<SearchResult, String> {
        // TODO: move away from struct, replace self with Arc, make locks shorter
        let t0 = std::time::Instant::now();
        let embedding_mb = fetch_embedding::get_embedding_with_retry(
            self.vecdb_emb_client.clone(),
            &self.constants.endpoint_embeddings_style,
            &self.constants.model_name,
            &self.constants.endpoint_embeddings_template,
            vec![query.clone()],
            &self.cmdline.api_key,
            5
        ).await;
        if embedding_mb.is_err() {
            return Err(embedding_mb.unwrap_err().to_string());
        }
        info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

        let mut handler_locked = self.vecdb_handler.lock().await;
        let t1 = std::time::Instant::now();
        let mut results = match handler_locked.search(&embedding_mb.unwrap()[0], top_n, vecdb_scope_filter_mb).await {
            Ok(res) => res,
            Err(err) => { return Err(err.to_string()) }
        };
        info!("search itself {:.3}s", t1.elapsed().as_secs_f64());
        let mut dist0 = 0.0;
        for rec in results.iter_mut() {
            if dist0 == 0.0 {
                dist0 = rec.distance.abs();
            }
            let last_35_chars = crate::nicer_logs::last_n_chars(&rec.file_path.display().to_string(), 35);
            rec.usefulness = 100.0 - 75.0 * ((rec.distance.abs() - dist0) / (dist0 + 0.01)).max(0.0).min(1.0);
            info!("distance {:.3} -> useful {:.1}, found {}:{}-{}", rec.distance, rec.usefulness, last_35_chars, rec.start_line, rec.end_line);
        }
        Ok(
            SearchResult {
                query_text: query,
                results,
            }
        )
    }
}
