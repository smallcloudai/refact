use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use tracing::info;
use tokio::sync::Mutex as AMutex;
use tokio::task::JoinHandle;
use crate::global_context::{CommandLine, GlobalContext};
use tokio::sync::RwLock as ARwLock;
use tower_lsp::lsp_types::WorkspaceFolder;
use tracing::error;
use crate::background_tasks::BackgroundTasksHolder;

use crate::fetch_embedding;
use crate::vecdb;
use crate::vecdb::{file_filter};
use crate::vecdb::handler::{VecDBHandler, VecDBHandlerRef};
use crate::vecdb::vectorizer_service::FileVectorizerService;
use crate::vecdb::structs::{SearchResult, VecdbSearch, VecDbStatus};


#[derive(Debug)]
pub struct VecDb {
    vecdb_handler: VecDBHandlerRef,
    retriever_service: Arc<AMutex<FileVectorizerService>>,
    cmdline: CommandLine,

    model_name: String,
    endpoint_template: String,
    endpoint_embeddings_style: String,
}


#[derive(Debug, Serialize)]
pub struct VecDbCaps {
    functions: Vec<String>,
}

struct VecDbParams {
    default_embeddings_model: String,
    endpoint_embeddings_template: String,
    endpoint_embeddings_style: String,
    size_embeddings: i32,
}

async fn vecdb_test_request(
    vecdb: &VecDb
) -> Result<(), String> {
    let search_result = vecdb.search("".to_string(), 3).await;
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
    vdb_params: VecDbParams,
) -> Result<(), String> {
    info!("vecdb: attempting to launch");

    let (cache_dir, cmdline) = {
        let gcx_locked = global_context.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.cmdline.clone())
    };
    let vec_db_mb = match VecDb::init(
        &cache_dir, cmdline.clone(),
        vdb_params.size_embeddings, 60, 512, 1024,
        vdb_params.default_embeddings_model,
        vdb_params.endpoint_embeddings_template,
        vdb_params.endpoint_embeddings_style,
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

    {
        let mut gcx_locked = global_context.write().await;

        if let Some(folders) = gcx_locked.lsp_backend_document_state.workspace_folders.clone().read().await.clone() {
            let mut vec_db_lock = gcx_locked.vec_db.lock().await;
            if let Some(ref mut db) = *vec_db_lock {
                db.init_folders(folders).await;
            }
        }
        let mut tasks = vec_db.start_background_tasks().await;
        tasks.extend(vec![tokio::spawn(vecdb::file_watcher_service::file_watcher_task(global_context.clone()))]);
        background_tasks.extend(tasks);

        gcx_locked.vec_db = Arc::new(AMutex::new(Some(vec_db)));

    }
    info!("vecdb: launch complete");
    Ok(())
}

async fn proceed_vecdb_reload(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> (bool, Option<VecDbParams>) {
    let caps = match crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await {
        Ok(caps) => caps,
        Err(_) => { return (false, None) }
    };

    let vdb_params = {
        let caps_locked = caps.read().unwrap();
        VecDbParams {
            default_embeddings_model: caps_locked.default_embeddings_model.clone(),
            endpoint_embeddings_template: caps_locked.endpoint_embeddings_template.clone(),
            endpoint_embeddings_style: caps_locked.endpoint_embeddings_style.clone(),
            size_embeddings: caps_locked.size_embeddings.clone(),
        }
    };

    if vdb_params.default_embeddings_model.is_empty() || vdb_params.endpoint_embeddings_template.is_empty() {
        error!("vecdb launch failed: default_embeddings_model.is_empty() || endpoint_embeddings_template.is_empty()");
        return (false, None);
    }


    match *global_context.write().await.vec_db.lock().await {
        None => {}
        Some(ref db) => {
            if db.model_name == vdb_params.default_embeddings_model &&
                db.endpoint_template == vdb_params.endpoint_embeddings_template &&
                db.endpoint_embeddings_style == vdb_params.endpoint_embeddings_style {
                return (false, None);
            }
        }
    }

    return (true, Some(vdb_params));
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
        let (proceed, vdb_params_mb) = proceed_vecdb_reload(global_context.clone()).await;
        if proceed && vdb_params_mb.is_some() {
            background_tasks.abort().await;
            background_tasks = BackgroundTasksHolder::new(vec![]);

            match create_vecdb(
                global_context.clone(),
                &mut background_tasks,
                vdb_params_mb.unwrap(),
            ).await{
                Ok(_) => {}
                Err(err) => {error!("vecdb: reload failed: {}", err);}
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

impl VecDb {
    pub async fn init(
        cache_dir: &PathBuf,
        cmdline: CommandLine,
        embedding_size: i32,
        cooldown_secs: u64,
        splitter_window_size: usize,
        splitter_soft_limit: usize,

        model_name: String,
        endpoint_template: String,
        endpoint_embeddings_style: String,
    ) -> Result<VecDb, String> {
        let handler = match VecDBHandler::init(cache_dir, &model_name, embedding_size).await {
            Ok(res) => res,
            Err(err) => { return Err(err) }
        };
        let vecdb_handler = Arc::new(AMutex::new(handler));
        let retriever_service = Arc::new(AMutex::new(FileVectorizerService::new(
            vecdb_handler.clone(),
            cooldown_secs,
            splitter_window_size,
            splitter_soft_limit,

            model_name.clone(),
            cmdline.api_key.clone(),
            endpoint_embeddings_style.clone(),
            endpoint_template.clone(),
        ).await));

        Ok(VecDb {
            vecdb_handler,
            retriever_service,
            cmdline: cmdline.clone(),

            model_name,
            endpoint_template,
            endpoint_embeddings_style,
        })
    }

    pub async fn start_background_tasks(&self) -> Vec<JoinHandle<()>> {
        info!("vecdb: start_background_tasks");
        return self.retriever_service.lock().await.start_background_tasks().await;
    }

    pub async fn add_or_update_file(&mut self, file_path: PathBuf, force: bool) {
        self.retriever_service.lock().await.process_file(file_path, force).await;
    }

    pub async fn add_or_update_files(&self, file_paths: Vec<PathBuf>, force: bool) {
        self.retriever_service.lock().await.process_files(file_paths, force).await;
    }

    pub async fn remove_file(&self, file_path: &PathBuf) {
        self.vecdb_handler.lock().await.remove(file_path).await;
    }

    pub async fn get_status(&self) -> Result<VecDbStatus, String> {
        self.retriever_service.lock().await.status().await
    }

    pub async fn init_folders(&self, folders: Vec<WorkspaceFolder>) {
        let files = file_filter::retrieve_files_by_proj_folders(
            folders.iter().map(|x| PathBuf::from(x.uri.path())).collect()
        ).await;
        self.add_or_update_files(files, true).await;
        info!("vecdb: init_folders complete");
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
        let embedding_mb = fetch_embedding::try_get_embedding(
            &self.endpoint_embeddings_style,
            &self.model_name,
            &self.endpoint_template,
            query.clone(),
            &self.cmdline.api_key,
            3
        ).await;
        if embedding_mb.is_err() {
            return Err("Failed to get embedding".to_string());
        }
        let mut binding = self.vecdb_handler.lock().await;

        let results = binding.search(embedding_mb.unwrap(), top_n).await.unwrap();
        binding.update_record_statistic(results.clone()).await;
        Ok(
            SearchResult {
                query_text: query,
                results,
            }
        )
    }
}
