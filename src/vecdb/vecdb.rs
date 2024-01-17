use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
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


pub async fn create_vecdb(
    default_embeddings_model: String,
    endpoint_embeddings_template: String,
    endpoint_embeddings_style: String,
    size_embeddings: i32,

    cmdline: &CommandLine,
    cache_dir: &PathBuf,
) -> Option<VecDb> {
    let vec_db = match VecDb::init(
        &cache_dir, cmdline.clone(),
        size_embeddings, 60, 512, 1024,
        default_embeddings_model.clone(),
        endpoint_embeddings_template.clone(),
        endpoint_embeddings_style.clone(),
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
            None
        }
    };
    vec_db
}

pub async fn vecdb_background_reload(
    global_context: Arc<ARwLock<GlobalContext>>,
) {
    let mut background_tasks = BackgroundTasksHolder::new(vec![]);
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        let (cache_dir, cmdline) = {
            let gcx_locked = global_context.read().await;
            let cache_dir = gcx_locked.cache_dir.clone();
            (&cache_dir.clone(), &gcx_locked.cmdline.clone())
        };

        let caps_mb = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone(), 0).await;

        if caps_mb.is_err() || !cmdline.vecdb {
            continue;
        }

        let (
            default_embeddings_model,
            endpoint_embeddings_template,
            endpoint_embeddings_style,
            size_embeddings,
        ) = {
            let caps = caps_mb.unwrap();
            let caps_locked = caps.read().unwrap();
            (
                caps_locked.default_embeddings_model.clone(),
                caps_locked.endpoint_embeddings_template.clone(),
                caps_locked.endpoint_embeddings_style.clone(),
                caps_locked.size_embeddings.clone(),
            )
        };

        if default_embeddings_model.is_empty() || endpoint_embeddings_template.is_empty() {
            error!("vecd launch failed: default_embeddings_model.is_empty() || endpoint_embeddings_template.is_empty()");
            continue;
        }


        match *global_context.write().await.vec_db.lock().await {
            None => {}
            Some(ref db) => {
                if db.model_name == default_embeddings_model &&
                    db.endpoint_template == endpoint_embeddings_template &&
                    db.endpoint_embeddings_style == endpoint_embeddings_style {
                    continue;
                }
            }
        }

        info!("vecdb: attempting to launch");

        background_tasks.abort().await;
        background_tasks = BackgroundTasksHolder::new(vec![]);

        let vecdb_mb = create_vecdb(
            default_embeddings_model.clone(),
            endpoint_embeddings_template,
            endpoint_embeddings_style,
            size_embeddings,

            cmdline,
            cache_dir
        ).await;

        if vecdb_mb.is_none() {
            continue;
        }
        let vecdb = vecdb_mb.unwrap();

        let search_result = vecdb.search("".to_string(), 3).await;
        match search_result {
            Ok(_) => {
                info!("vecdb: test search complete")
            }
            Err(_) => {
                error!("vecdb: test search failed");
                continue;
            }
        }

        {
            let mut gcx_locked = global_context.write().await;
            gcx_locked.vec_db = Arc::new(AMutex::new(Some(vecdb)));
            info!("vecdb is launched successfully");

            background_tasks.extend(match *gcx_locked.vec_db.lock().await {
                Some(ref db) => {
                    let mut tasks = db.start_background_tasks().await;
                    tasks.push(
                        tokio::spawn(vecdb::file_watcher_service::file_watcher_task(global_context.clone()))
                    );
                    tasks
                }
                None => vec![]
            });
            {
                if let Some(folders) = gcx_locked.lsp_backend_document_state.workspace_folders.clone().read().await.clone() {
                    let mut vec_db_lock = gcx_locked.vec_db.lock().await;
                    if let Some(ref mut db) = *vec_db_lock {
                        db.init_folders(folders).await;
                    }
                }
            }
        }
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
