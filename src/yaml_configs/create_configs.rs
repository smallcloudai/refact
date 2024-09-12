use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock as ARwLock;
use tracing::{info, warn};
use crate::caps::SIMPLE_CAPS;
use crate::global_context::GlobalContext;
use crate::yaml_configs::customization_compiled_in::COMPILED_IN_INITIAL_USER_YAML;


pub async fn yaml_configs_try_create_all(gcx: Arc<ARwLock<GlobalContext>>) {
    match yaml_bring_your_own_key_exists_or_create(gcx.clone()).await {
        Ok(_) => (),
        Err(e) => warn!("{}", e)
    }
    match yaml_integrations_exists_or_create(gcx.clone()).await {
        Ok(_) => (),
        Err(e) => warn!("{}", e)
    }
    match yaml_customization_exists_or_create(gcx.clone()).await {
        Ok(_) => (),
        Err(e) => warn!("{}", e)
    }
}

pub async fn yaml_bring_your_own_key_exists_or_create(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let path = cache_dir.join("bring-your-own-key.yaml");
    if !path.exists() {
        let mut file = std::fs::File::create(&path)
            .map_err(|e| format!("Failed to create bring-your-own-key.yaml: {}", e))?;
        file.write_all(SIMPLE_CAPS.as_bytes())
            .map_err(|e| format!("Failed to write into bring-your-own-key.yaml: {}", e))?;
        info!("Created bring-your-own-key.yaml: {}", path.display());
    }
    Ok(path)
}

pub async fn yaml_integrations_read(gcx: Arc<ARwLock<GlobalContext>>) -> Result<String, String> {
    let integrations_yaml = yaml_integrations_exists_or_create(gcx).await?;
    fs::read_to_string(&integrations_yaml).await.map_err(|e|format!("Failed to read integrations.yaml: {}", e))
}

async fn yaml_integrations_exists_or_create(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let path = cache_dir.join("integrations.yaml");
    if !path.exists() {
        // todo: create integrations yaml
        Err("integrations.yaml does not exist and cannot be created automatically".to_string())?;
        info!("Created integrations.yaml: {}", path.display());
    }
    Ok(path)
}

pub async fn yaml_customization_exists_or_create(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let user_config_path = cache_dir.join("customization.yaml");
    if !user_config_path.exists() {
        let mut file = std::fs::File::create(&user_config_path)
            .map_err(|e| format!("Failed to create customization.yaml: {}", e))?;
        file.write_all(COMPILED_IN_INITIAL_USER_YAML.as_bytes())
            .map_err(|e| format!("Failed to write into customization.yaml: {}", e))?;
        info!("Created customization.yaml: {}", user_config_path.display());
    }
    Ok(user_config_path)
}
