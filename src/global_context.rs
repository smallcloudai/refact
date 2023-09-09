use tracing::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;

use crate::recommendations::CodeAssistantRecommendations;


pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub recommendations: Arc<StdRwLock<CodeAssistantRecommendations>>,
}


pub fn create_global_context(
    home_dir: PathBuf,
    rec: Arc<StdRwLock<CodeAssistantRecommendations>>
) -> Arc<ARwLock<GlobalContext>> {
    let cache_dir = home_dir.join(".cache/refact");
    info!("cache dir: {}", cache_dir.display());
    Arc::new(ARwLock::new(GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir: cache_dir,
        tokenizer_map: HashMap::new(),
        recommendations: rec,
    }))
}
