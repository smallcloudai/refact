use tracing::info;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use std::sync::RwLock as StdRwLock;
use std::collections::HashMap;
use tokenizers::Tokenizer;


pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
}


pub fn create_global_context(home_dir: PathBuf) -> Arc<ARwLock<GlobalContext>> {
    let cache_dir = home_dir.join(".cache/refact");
    info!("cache dir: {}", cache_dir.display());
    Arc::new(ARwLock::new(GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir: cache_dir,
        tokenizer_map: HashMap::new(),
    }))
}
