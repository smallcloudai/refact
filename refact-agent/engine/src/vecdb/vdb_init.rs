use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AMutex;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::caps::get_custom_embedding_api_key;
use crate::global_context::{CommandLine, GlobalContext};
use crate::vecdb::vdb_highlev::VecDb;
use crate::vecdb::vdb_structs::{VecdbConstants, VecdbSearch};
use tokio::sync::RwLock as ARwLock;

pub struct VecDbInitConfig {
    pub max_attempts: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
    pub test_search_after_init: bool,
}

impl Default for VecDbInitConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            max_delay_ms: 10000,
            backoff_factor: 2.0,
            test_search_after_init: true,
        }
    }
}

#[derive(Debug)]
pub enum VecDbInitError {
    ApiKeyError(String),
    InitializationError(String),
    TestSearchError(String),
}

impl std::fmt::Display for VecDbInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VecDbInitError::ApiKeyError(msg) => write!(f, "API key error: {}", msg),
            VecDbInitError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            VecDbInitError::TestSearchError(msg) => write!(f, "Test search error: {}", msg),
        }
    }
}

pub async fn init_vecdb_fail_safe(
    cache_dir: &PathBuf,
    cmdline: CommandLine,
    constants: VecdbConstants,
    api_key: &String,
    init_config: VecDbInitConfig,
) -> Result<VecDb, VecDbInitError> {
    let mut attempt: usize = 0;
    let mut delay = Duration::from_millis(init_config.initial_delay_ms);
    
    loop {
        attempt += 1;
        info!("VecDb init attempt {}/{}", attempt, init_config.max_attempts);
        match VecDb::init(cache_dir, cmdline.clone(), constants.clone()).await {
            Ok(vecdb) => {
                info!("Successfully initialized VecDb on attempt {}", attempt);
                
                if init_config.test_search_after_init {
                    match vecdb_test_search(&vecdb, api_key).await {
                        Ok(_) => {
                            info!("VecDb test search successful");
                            return Ok(vecdb);
                        },
                        Err(err) => {
                            warn!("VecDb test search failed: {}", err);
                            if attempt >= init_config.max_attempts {
                                return Err(VecDbInitError::TestSearchError(err));
                            }
                        }
                    }
                } else {
                    return Ok(vecdb);
                }
            },
            Err(err) => {
                if attempt >= init_config.max_attempts {
                    error!("VecDb initialization failed after {} attempts. Last error: {}", attempt, err);
                    return Err(VecDbInitError::InitializationError(err));
                } else {
                    warn!(
                        "VecDb initialization attempt {} failed with error: {}. Retrying in {:?}...",
                        attempt, err, delay
                    );
                    sleep(delay).await;
                    
                    let new_delay_ms = (delay.as_millis() as f64 * init_config.backoff_factor) as u64;
                    delay = Duration::from_millis(new_delay_ms.min(init_config.max_delay_ms));
                }
            }
        }
    }
}

async fn vecdb_test_search(vecdb: &VecDb, api_key: &String) -> Result<(), String> {
    let test_query = "test query".to_string();
    let top_n = 3;
    let filter = None;
    
    match VecdbSearch::vecdb_search(vecdb, test_query, top_n, filter, api_key, None).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Test search failed: {}", e)),
    }
}

pub async fn initialize_vecdb_with_context(
    gcx: Arc<ARwLock<GlobalContext>>,
    constants: VecdbConstants,
    init_config: Option<VecDbInitConfig>,
) -> Result<crate::background_tasks::BackgroundTasksHolder, VecDbInitError> {
    let api_key = match get_custom_embedding_api_key(gcx.clone()).await {
        Ok(key) => key,
        Err(err) => return Err(VecDbInitError::ApiKeyError(err.message)),
    };
    
    let (cache_dir, cmdline) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.cache_dir.clone(), gcx_locked.cmdline.clone())
    };
    
    let base_dir_cache = match cmdline.vecdb_force_path.as_str() {
        "" => cache_dir,
        path => PathBuf::from(path)
    };
    
    let config = init_config.unwrap_or_default();
    let vec_db = init_vecdb_fail_safe(
        &base_dir_cache,
        cmdline.clone(),
        constants.clone(),
        &api_key,
        config,
    ).await?;
    
    info!("VecDb initialization successful, updating global context");
    {
        let mut gcx_locked = gcx.write().await;
        gcx_locked.vecdb = Some(Arc::new(AMutex::new(vec_db)));
        gcx_locked.vec_db_error = "".to_string();
    }

    info!("Enqueuing workspace files for vectorization");
    crate::files_in_workspace::enqueue_all_files_from_workspace_folders(gcx.clone(), true, true).await;
    crate::files_in_jsonl::enqueue_all_docs_from_jsonl_but_read_first(gcx.clone(), true, true).await;

    info!("Starting background tasks for vectorization");
    let background_tasks = {
        let tasks = crate::vecdb::vectorizer_service::vectorizer_service_init_and_start(
            gcx.clone(), constants
        ).await.map_err(|x| VecDbInitError::InitializationError(x))?;
        crate::background_tasks::BackgroundTasksHolder::new(tasks)
    };
    
    info!("VecDb initialization and setup complete");
    Ok(background_tasks)
}