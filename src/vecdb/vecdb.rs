use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tracing::{info, error};

use async_trait::async_trait;
use serde::Serialize;
use tokio::task::JoinHandle;
use crate::global_context::{CommandLine, GlobalContext};
use crate::background_tasks::BackgroundTasksHolder;

use crate::fetch_embedding;
use crate::vecdb::handler::VecDBHandler;
use crate::vecdb::vectorizer_service::FileVectorizerService;
use crate::vecdb::structs::{SearchResult, VecdbSearch, VecDbStatus, VecdbConstants};

fn vecdb_constants(
    caps: Arc<StdRwLock<crate::caps::CodeAssistantCaps>>,
) -> VecdbConstants {
    let caps_locked = caps.read().unwrap();
    VecdbConstants {
        model_name: caps_locked.default_embeddings_model.clone(),
        embedding_size: caps_locked.size_embeddings.clone(),
        endpoint_embeddings_template: caps_locked.endpoint_embeddings_template.clone(),
        endpoint_embeddings_style: caps_locked.endpoint_embeddings_style.clone(),
        cooldown_secs: 20,
        splitter_window_size: 512,
        splitter_soft_limit: 1024,
    }
}

#[derive(Debug)]
pub struct VecDb {
    vecdb_emb_client: Arc<AMutex<reqwest::Client>>,
    vecdb_handler: Arc<AMutex<VecDBHandler>>,
    vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    cmdline: CommandLine,  // TODO: take from command line what's needed, don't store a copy 
    constants: VecdbConstants,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileSearchResult {
    pub file_path: String,
    pub file_text: String,
}

#[derive(Debug, Serialize)]
pub struct VecDbCaps {
    functions: Vec<String>,
}

async fn vecdb_test_request(
    vecdb: &VecDb
) -> Result<(), String> {
    let search_result = vecdb.search("test query".to_string(), 3).await;
    match search_result {
        Ok(_) => {
            Ok(())
        }
        Err(_) => {
            error!("vecdb: test search failed");
            Err("vecdb: test search failed".to_string())
        }
    }
}

async fn create_vecdb(
    global_context: Arc<ARwLock<GlobalContext>>,
    background_tasks: &mut BackgroundTasksHolder,
    constants: VecdbConstants,
) -> Result<(), String> {
    info!("vecdb: attempting to launch");

    let (cache_dir, cmdline) = {
        let gcx_locked = global_context.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.cmdline.clone())
    };
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

    let files_jsonl_path = global_context.read().await.cmdline.files_jsonl_path.clone();
    let files = match crate::files_in_jsonl::parse_jsonl(&files_jsonl_path).await {
        Ok(lst) => lst,
        Err(err) => {
            error!("failed to parse {}: {}", files_jsonl_path, err);
            vec![]
        }
    };
    vec_db.vectorizer_enqueue_files(&files, true).await;
    let tasks = vec_db.vecdb_start_background_tasks().await;
    background_tasks.extend(tasks);

    {
        let mut gcx_locked = global_context.write().await;
        gcx_locked.vec_db = Arc::new(AMutex::new(Some(vec_db)));
    }

    Ok(())
}

async fn do_i_need_to_reload_vecdb(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> (bool, Option<VecdbConstants>) {
    let caps = match crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await {
        Ok(caps) => caps,
        Err(e) => {
            // This branch makes caps error disappear, unless we print it right here:
            info!("vecdb: no caps, will not start or reload vecdb, the error was: {}", e);
            return (false, None)
        }
    };
    let consts = vecdb_constants(caps);

    if consts.model_name.is_empty() || consts.endpoint_embeddings_template.is_empty() {
        error!("vecdb launch failed: default_embeddings_model.is_empty() || endpoint_embeddings_template.is_empty()");
        return (false, None);
    }

    match *global_context.write().await.vec_db.lock().await {
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
    global_context: Arc<ARwLock<GlobalContext>>,
) {
    let cmd_line = global_context.read().await.cmdline.clone();
    if !cmd_line.vecdb {
        return;
    }
    let mut background_tasks = BackgroundTasksHolder::new(vec![]);
    loop {
        let (need_reload, consts) = do_i_need_to_reload_vecdb(global_context.clone()).await;
        if need_reload && consts.is_some() {
            background_tasks.abort().await;
            background_tasks = BackgroundTasksHolder::new(vec![]);
            match create_vecdb(
                global_context.clone(),
                &mut background_tasks,
                consts.unwrap(),
            ).await {
                Ok(_) => {
                    crate::files_in_workspace::enqueue_all_files_from_workspace_folders(global_context.clone()).await;
                }
                Err(err) => {
                    error!("vecdb: init failed: {}", err);
                    // global_context.vec_db stays None, the rest of the system continues working
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
        let handler = match VecDBHandler::init(cache_dir, &constants.model_name, constants.embedding_size).await {
            Ok(res) => res,
            Err(err) => { return Err(err) }
        };
        let vecdb_handler = Arc::new(AMutex::new(handler));
        let vectorizer_service = Arc::new(AMutex::new(FileVectorizerService::new(
            vecdb_handler.clone(),
            constants.clone(),
            cmdline.api_key.clone(),
        ).await));
        Ok(VecDb {
            vecdb_emb_client: Arc::new(AMutex::new(reqwest::Client::new())),
            vecdb_handler,
            vectorizer_service,
            cmdline: cmdline.clone(),
            constants: constants.clone(),
        })
    }

    pub async fn vecdb_start_background_tasks(&self) -> Vec<JoinHandle<()>> {
        info!("vecdb: start_background_tasks");
        return self.vectorizer_service.lock().await.vecdb_start_background_tasks(self.vecdb_emb_client.clone()).await;
    }

    pub async fn vectorizer_enqueue_files(&self, file_paths: &Vec<PathBuf>, force: bool) {
        self.vectorizer_service.lock().await.vectorizer_enqueue_files(file_paths, force).await;
    }

    pub async fn remove_file(&self, file_path: &PathBuf) {
        self.vecdb_handler.lock().await.remove(file_path).await;
    }

    pub async fn get_status(&self) -> Result<VecDbStatus, String> {
        self.vectorizer_service.lock().await.status().await
    }

    pub async fn get_indexed_file_paths(&self) -> Arc<AMutex<Vec<PathBuf>>> {
        return self.vecdb_handler.lock().await.get_indexed_file_paths().await;
    }

    pub async fn get_file_orig_text(&self, file_path: String) -> FileSearchResult {
        let text = self.vecdb_handler.lock().await.get_file_orig_text(file_path.clone()).await;
        FileSearchResult { file_path, file_text: text }
    }

    pub async fn caps(&self) -> VecDbCaps {
        VecDbCaps {
            functions: vec!["@workspace".to_string()],
        }
    }
}


#[async_trait]
impl VecdbSearch for VecDb {
    async fn search(&self, query: String, top_n: usize) -> Result<SearchResult, String> {
        let t0 = std::time::Instant::now();
        let embedding_mb = fetch_embedding::try_get_embedding(
            self.vecdb_emb_client.clone(),
            &self.constants.endpoint_embeddings_style,
            &self.constants.model_name,
            &self.constants.endpoint_embeddings_template,
            query.clone(),
            &self.cmdline.api_key,
            5
        ).await;
        if embedding_mb.is_err() {
            return Err("Failed to get embedding".to_string());
        }
        info!("search query {:?}, it took {:.3}s to vectorize the query", query, t0.elapsed().as_secs_f64());

        let mut handler_locked = self.vecdb_handler.lock().await;
        let t1 = std::time::Instant::now();
        let results = match handler_locked.search(embedding_mb.unwrap(), top_n).await {
            Ok(res) => res,
            Err(_) => {return  Err("error during search occurred".to_string()) },
        };
        info!("search itself {:.3}s", t1.elapsed().as_secs_f64());
        for rec in results.iter() {
            let last_30_chars = crate::nicer_logs::last_n_chars(&rec.file_path.display().to_string(), 30);
            info!("distance {:.3}, found {}:{}-{}, ", rec.distance, last_30_chars, rec.start_line, rec.end_line);
        }
        let t2 = std::time::Instant::now();
        handler_locked.update_record_statistic(results.clone()).await;
        info!("update_record_statistic {:.3}s", t2.elapsed().as_secs_f64());
        Ok(
            SearchResult {
                query_text: query,
                results,
            }
        )
    }
}
