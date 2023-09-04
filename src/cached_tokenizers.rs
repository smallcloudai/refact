use reqwest::header::AUTHORIZATION;
use std::fmt::Display;
use tracing::{error, info};
use std::collections::HashMap;
use tokenizers::Tokenizer;
use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;


pub type Result<T> = std::result::Result<T, Error>;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

fn tokenizer_error<E: Display>(err: E) -> Error {
    let err_msg = err.to_string();
    error!("tokenizer error: {}", err_msg);
    Error {
        message: err_msg.into(),
        data: None,
    }
}

pub async fn download_tokenizer_file(
    http_client: &reqwest::Client,
    model: &str,
    api_token: Option<&String>,
    to: impl AsRef<Path>,
) -> Result<()> {
    if to.as_ref().exists() {
        return Ok(());
    }
    info!("Downloading tokenizer for \"{}\"...", model);
    tokio::fs::create_dir_all(
        to.as_ref()
            .parent()
            .ok_or_else(|| tokenizer_error("tokenizer path has no parent"))?,
        )
        .await
        .map_err(tokenizer_error)?;
    let mut req = http_client.get(format!(
        "https://huggingface.co/{model}/resolve/main/tokenizer.json"
    ));
    if let Some(api_token) = api_token {
        req = req.header(AUTHORIZATION, format!("Bearer {api_token}"))
    }
    let res = req
        .send()
        .await
        .map_err(tokenizer_error)?
        .error_for_status()
        .map_err(tokenizer_error)?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(to)
        .await
        .map_err(tokenizer_error)?;
    file.write_all(&res.bytes().await.map_err(tokenizer_error)?)
        .await
        .map_err(tokenizer_error)?;
    Ok(())
}

pub async fn get_tokenizer(
    model: &str,
    tokenizer_map: &mut HashMap<String, Arc<RwLock<Tokenizer>>>,
    http_client: &reqwest::Client,
    cache_dir: &Path,
    api_token: Option<&String>,
) -> Result<Arc<RwLock<Tokenizer>>> {
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
            download_tokenizer_file(http_client, model, api_token, &path).await?;
            let tokenizer = Tokenizer::from_file(path).map_err(tokenizer_error)?;
            let arc = Arc::new(RwLock::new(tokenizer));
            tokenizer_map.insert(model.to_owned(), arc.clone());
            Ok(arc)
        }
    }
}
