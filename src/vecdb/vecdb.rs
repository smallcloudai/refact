use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::vecdb::handler::{VecDBHandler, VecDBHandlerRef};
use crate::vecdb::req_client::get_embedding;
use crate::vecdb::vectorizer_service::FileVectorizerService;
use crate::vecdb::structs::{SearchResult, VecdbSearch, VecDbStatus};

#[derive(Debug)]
pub struct VecDb {
    vecdb_handler: VecDBHandlerRef,
    retriever_service: Arc<Mutex<FileVectorizerService>>,
    embedding_model_name: String,
    cmdline: crate::global_context::CommandLine,
}


impl VecDb {
    pub async fn init(
        cache_dir: PathBuf,
        cmdline: crate::global_context::CommandLine,
        embedding_size: i32,
        cooldown_secs: u64,
        splitter_window_size: usize,
        splitter_soft_limit: usize,
        embedding_model_name: String,
    ) -> Result<VecDb, String> {
        let handler = match VecDBHandler::init(cache_dir, embedding_size).await {
            Ok(res) => res,
            Err(err) => { return Err(err) }
        };
        let vecdb_handler = Arc::new(Mutex::new(handler));
        let retriever_service = Arc::new(Mutex::new(FileVectorizerService::new(
            vecdb_handler.clone(), cooldown_secs, splitter_window_size, splitter_soft_limit,
            embedding_model_name.clone(), cmdline.api_key.clone(),
        ).await));

        Ok(VecDb {
            vecdb_handler,
            retriever_service,
            embedding_model_name,
            cmdline,
        })
    }

    pub async fn start_background_tasks(&self) -> Vec<JoinHandle<()>> {
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
}


#[async_trait]
impl VecdbSearch for VecDb {
    async fn search(&self, query: String, top_n: usize) -> Result<SearchResult, String> {
        let embedding = get_embedding(
            query.clone(), &self.embedding_model_name, self.cmdline.api_key.clone(),
        ).await.unwrap();
        match embedding {
            Ok(vector) => {
                let mut binding = self.vecdb_handler.lock().await;
                let results = binding.search(vector, top_n).await.unwrap();
                binding.update_record_statistic(results.clone()).await;
                Ok(
                    SearchResult {
                        query_text: query,
                        results: results,
                    }
                )
            }
            Err(_) => {
                return Err("Failed to get embedding".to_string());
            }
        }
    }
}
