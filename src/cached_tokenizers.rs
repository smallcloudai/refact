use reqwest::header::AUTHORIZATION;
use tracing::info;
use std::collections::HashMap;
use tokenizers::Tokenizer;
use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub async fn download_tokenizer_file(
    http_client: &reqwest::Client,
    model: &str,
    api_token: Option<String>,
    to: impl AsRef<Path>,
) -> Result<(), String> {
    if to.as_ref().exists() {
        return Ok(());
    }
    info!("Downloading tokenizer for \"{}\"...", model);
    tokio::fs::create_dir_all(
            to.as_ref().parent().ok_or_else(|| "tokenizer path has no parent")?,
        )
        .await
        .map_err(|e| format!("failed to create parent dir: {}", e))?;
    let mut req = http_client.get(format!(
        "https://huggingface.co/{model}/resolve/main/tokenizer.json"
    ));
    if let Some(api_token) = api_token {
        req = req.header(AUTHORIZATION, format!("Bearer {api_token}"))
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("failed to get response: {}", e))?
        .error_for_status()
        .map_err(|e| format!("failed to get response: {}", e))?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(to)
        .await
        .map_err(|e| format!("failed to open file: {}", e))?;
    file.write_all(&res.bytes().await
        .map_err(|e| format!("failed to fetch bytes: {}", e))?
    ).await.map_err(|e| format!("failed to write to file: {}", e))?;
    Ok(())
}

pub async fn get_tokenizer(
    tokenizer_map: &mut HashMap<String, Arc<StdRwLock<Tokenizer>>>,
    model: &str,
    http_client: reqwest::Client,
    cache_dir: &Path,
    api_token: Option<String>,
) -> Result<Arc<StdRwLock<Tokenizer>>, String> {
    // tokenizer_path: Option<&String>,
    // if model.starts_with("http://") || model.starts_with("https://") {
    //     let tokenizer = match tokenizer_path {
    //         Some(path) => Tokenizer::from_file(path).map_err(tokenizer_error)?,
    //         None => return Err(tokenizer_error("`tokenizer_path` is null")),
    //     };
    //     Ok(tokenizer)
    // } else {
    match tokenizer_map.get(model) {
        Some(arc) => Ok(arc.clone()),
        None => {
            let tokenizer_cache_dir = PathBuf::from(cache_dir); //.join("tokenizers");
            tokio::fs::create_dir_all(&tokenizer_cache_dir)
                .await
                .expect("failed to create cache dir");
            let path = tokenizer_cache_dir.join(model).join("tokenizer.json");
            download_tokenizer_file(&http_client, model, api_token, &path).await?;
            let tokenizer = Tokenizer::from_file(path).map_err(|e| format!("failed to load tokenizer: {}", e))?;
            let arc = Arc::new(StdRwLock::new(tokenizer));
            tokenizer_map.insert(model.to_owned(), arc.clone());
            Ok(arc)
        }
    }
}
