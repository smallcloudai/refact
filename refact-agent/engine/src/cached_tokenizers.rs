use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokenizers::Tokenizer;
use reqwest::header::AUTHORIZATION;
use reqwest::Response;
use tracing::{error, info};
use uuid::Uuid;

use crate::custom_error::MapErrToString;
use crate::files_correction::canonical_path;
use crate::global_context::GlobalContext;
use crate::caps::{strip_model_from_finetune, BaseModelRecord};


async fn try_open_tokenizer(
    res: Response,
    to: impl AsRef<Path>,
) -> Result<(), String> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&to)
        .await
        .map_err(|e| format!("failed to open file: {}", e))?;
    file.write_all(&res.bytes().await
        .map_err(|e| format!("failed to fetch bytes: {}", e))?
    ).await.map_err(|e| format!("failed to write to file: {}", e))?;
    file.flush().await.map_err(|e| format!("failed to flush file: {}", e))?;
    info!("saved tokenizer to {}", to.as_ref().display());
    Ok(())
}

async fn download_tokenizer_file(
    http_client: &reqwest::Client,
    http_path: &str,
    api_token: &str,
    to: &Path,
) -> Result<(), String> {
    tokio::fs::create_dir_all(
        to.parent().ok_or_else(|| "tokenizer path has no parent")?,
    ).await.map_err(|e| format!("failed to create parent dir: {}", e))?;
    if to.exists() {
        return Ok(());
    }

    info!("downloading tokenizer from {}", http_path);
    let mut req = http_client.get(http_path);
    if api_token.to_lowercase().starts_with("hf_") {
        req = req.header(AUTHORIZATION, format!("Bearer {api_token}"))
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("failed to get response: {}", e))?
        .error_for_status()
        .map_err(|e| format!("failed to get response: {}", e))?;
    try_open_tokenizer(res, to).await?;
    Ok(())
}

fn check_json_file(path: &Path) -> bool {
    match Tokenizer::from_file(path) {
        Ok(_) => { true }
        Err(_) => { false }
    }
}

async fn try_download_tokenizer_file_and_open(
    http_client: &reqwest::Client,
    http_path: &str,
    api_token: &str,
    path: &Path,
) -> Result<(), String> {
    if path.exists() && check_json_file(path) {
        return Ok(());
    }

    let tmp_file = std::env::temp_dir().join(Uuid::new_v4().to_string());
    let tmp_path = tmp_file.as_path();

    for i in 0..15 {
        if i != 0 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let res = download_tokenizer_file(http_client, http_path, api_token, tmp_path).await;
        if res.is_err() {
            error!("failed to download tokenizer: {}", res.unwrap_err());
            continue;
        }

        let parent = path.parent();
        if parent.is_none() {
            error!("failed to download tokenizer: parent is not set");
            continue;
        }

        let res = tokio::fs::create_dir_all(parent.unwrap()).await;
        if res.is_err() {
            error!("failed to create parent dir: {}", res.unwrap_err());
            continue;
        }

        if !check_json_file(tmp_path) {
            error!("failed to download tokenizer: file is not a tokenizer");
            continue;
        }

        match tokio::fs::copy(tmp_path, path).await {
            Ok(_) => {
                info!("moved tokenizer to {}", path.display());
                return Ok(());
            },
            Err(_) => { continue; }
        }
    }
    Err("failed to download tokenizer".to_string())
}

pub async fn cached_tokenizer(
    global_context: Arc<ARwLock<GlobalContext>>,
    model_name: String,
    model_rec: &BaseModelRecord,
) -> Result<Arc<StdRwLock<Tokenizer>>, String> {
    let model_name = strip_model_from_finetune(&model_name);
    let tokenizer_download_lock: Arc<AMutex<bool>> = global_context.read().await.tokenizer_download_lock.clone();
    let _tokenizer_download_locked = tokenizer_download_lock.lock().await;

    let (client2, cache_dir, tokenizer_arc) = {
        let cx_locked = global_context.read().await;
        (cx_locked.http_client.clone(), cx_locked.cache_dir.clone(), cx_locked.tokenizer_map.clone().get(&model_name).cloned())
    };

    if tokenizer_arc.is_some() {
        return Ok(tokenizer_arc.unwrap().clone())
    }

    let (mut tok_file_path, tok_url) = match &model_rec.tokenizer {
        fake_tok if fake_tok.starts_with("fake://") => {
            todo!()
        }
        hf_tok if hf_tok.starts_with("hf://") => {
            let hf_tok = hf_tok.strip_prefix("hf://").unwrap();
            (PathBuf::new(), format!("https://huggingface.co/{hf_tok}/resolve/main/tokenizer.json"))
        }
        http_tok if http_tok.starts_with("http://") || http_tok.starts_with("https://") => {
            (PathBuf::new(), http_tok.to_string())
        }
        file_tok => {
            let file = if file_tok.starts_with("file://") {
                url::Url::parse(file_tok)
                    .and_then(|url| url.to_file_path().map_err(|_| url::ParseError::EmptyHost))
                    .map_err_with_prefix(format!("Invalid path URL {file_tok}:"))?
            } else {
                canonical_path(file_tok)
            };
            (canonical_path(file.to_string_lossy()), "".to_string())
        }
    };

    if tok_file_path.as_os_str().is_empty() {
        let tokenizer_cache_dir = std::path::PathBuf::from(cache_dir).join("tokenizers");
        let sanitized_model_name = model_name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        
        tok_file_path = tokenizer_cache_dir.join(&sanitized_model_name).join("tokenizer.json");

        try_download_tokenizer_file_and_open(&client2, &tok_url, &model_rec.api_key, &tok_file_path).await?;
    }
    
    info!("loading tokenizer \"{}\"", tok_file_path.display());
    let mut tokenizer = Tokenizer::from_file(tok_file_path)
        .map_err(|e| format!("failed to load tokenizer: {}", e))?;
    let _ = tokenizer.with_truncation(None);
    tokenizer.with_padding(None);
    let arc = Arc::new(StdRwLock::new(tokenizer));

    global_context.write().await.tokenizer_map.insert(model_name.clone(), arc.clone());
    Ok(arc)
}
