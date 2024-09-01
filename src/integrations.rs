use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
use tokio::fs;
use tracing::error;


pub async fn load_integrations(gcx: Arc<ARwLock<GlobalContext>>) -> String {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let path = cache_dir.join("integrations.yaml");
    match fs::read_to_string(path.clone()).await {
        Ok(content) => content,
        Err(_e) => {   // doesn't matter what the error is, trivial
            error!("cannot read {}, no integrations will be enabled", path.display());
            "".to_string()
        }
    }
}
