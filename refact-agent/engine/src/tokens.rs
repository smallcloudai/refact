use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokenizers::Tokenizer;
use reqwest::header::AUTHORIZATION;
use reqwest::Response;
use uuid::Uuid;

use crate::custom_error::MapErrToString;
use crate::files_correction::canonical_path;
use crate::global_context::GlobalContext;
use crate::caps::{default_hf_tokenizer_template, strip_model_from_finetune, BaseModelRecord};


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
    tracing::info!("saved tokenizer to {}", to.as_ref().display());
    Ok(())
}

async fn download_tokenizer_file(
    http_client: &reqwest::Client,
    http_path: &str,
    tokenizer_api_token: &str,
    to: &Path,
) -> Result<(), String> {
    tokio::fs::create_dir_all(
        to.parent().ok_or_else(|| "tokenizer path has no parent")?,
    ).await.map_err(|e| format!("failed to create parent dir: {}", e))?;
    if to.exists() {
        return Ok(());
    }

    tracing::info!("downloading tokenizer from {}", http_path);
    let mut req = http_client.get(http_path);
    
    if !tokenizer_api_token.is_empty() {
        req = req.header(AUTHORIZATION, format!("Bearer {tokenizer_api_token}"))
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
    tokenizer_api_token: &str,
    path: &Path,
) -> Result<(), String> {
    if path.exists() && check_json_file(path) {
        return Ok(());
    }

    let tmp_file = std::env::temp_dir().join(Uuid::new_v4().to_string());
    let tmp_path = tmp_file.as_path();
    
    // Track the last error message
    let mut last_error = String::from("");
    for i in 0..15 {
        if i != 0 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let res = download_tokenizer_file(http_client, http_path, tokenizer_api_token, tmp_path).await;
        if let Err(err_msg) = res {
            last_error = format!("failed to download tokenizer: {}", err_msg);
            tracing::error!("{last_error}");
            continue;
        }

        let parent = path.parent();
        if parent.is_none() {
            last_error = String::from("failed to download tokenizer: parent is not set");
            tracing::error!("{last_error}");
            continue;
        }

        let res = tokio::fs::create_dir_all(parent.unwrap()).await;
        if let Err(err_msg) = res {
            last_error = format!("failed to create parent dir: {}", err_msg);
            tracing::error!("{last_error}");
            continue;
        }

        if !check_json_file(tmp_path) {
            last_error = String::from("failed to download tokenizer: file is not a tokenizer");
            tracing::error!("{last_error}");
            continue;
        }

        match tokio::fs::copy(tmp_path, path).await {
            Ok(_) => {
                tracing::info!("moved tokenizer to {}", path.display());
                return Ok(());
            },
            Err(e) => { 
                last_error = format!("failed to copy tokenizer file: {}", e);
                tracing::error!("{last_error}");
                continue; 
            }
        }
    }
    Err(last_error)
}

pub async fn cached_tokenizer(
    global_context: Arc<ARwLock<GlobalContext>>,
    model_rec: &BaseModelRecord,
) -> Result<Option<Arc<Tokenizer>>, String> {
    let model_id = strip_model_from_finetune(&model_rec.id);
    let tokenizer_download_lock: Arc<AMutex<bool>> = global_context.read().await.tokenizer_download_lock.clone();
    let _tokenizer_download_locked = tokenizer_download_lock.lock().await;

    let (client2, cache_dir, tokenizer_in_gcx, hf_tokenizer_template) = {
        let cx_locked = global_context.read().await;
        let template = cx_locked.caps.clone().map(|caps| caps.hf_tokenizer_template.clone())
            .unwrap_or_else(default_hf_tokenizer_template);
        (cx_locked.http_client.clone(), cx_locked.cache_dir.clone(), cx_locked.tokenizer_map.clone().get(&model_id).cloned(), template)
    };

    if let Some(tokenizer) = tokenizer_in_gcx {
        return Ok(tokenizer)
    }

    let (mut tok_file_path, tok_url) = match &model_rec.tokenizer {
        empty_tok if empty_tok.is_empty() => return Err(format!("failed to load tokenizer: empty tokenizer for {model_id}")),
        fake_tok if fake_tok.starts_with("fake") => return Ok(None),
        hf_tok if hf_tok.starts_with("hf://") => {
            let hf_model = hf_tok.strip_prefix("hf://").unwrap();
            let url = hf_tokenizer_template.replace("$HF_MODEL", hf_model);
            (PathBuf::new(), url)
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
        let sanitized_model_id = model_id.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        
        tok_file_path = tokenizer_cache_dir.join(&sanitized_model_id).join("tokenizer.json");

        try_download_tokenizer_file_and_open(&client2, &tok_url, &model_rec.tokenizer_api_key, &tok_file_path).await?;
    }
    
    tracing::info!("loading tokenizer \"{}\"", tok_file_path.display());
    let mut tokenizer = Tokenizer::from_file(tok_file_path)
        .map_err(|e| format!("failed to load tokenizer: {}", e))?;
    let _ = tokenizer.with_truncation(None);
    tokenizer.with_padding(None);
    let arc = Some(Arc::new(tokenizer));

    global_context.write().await.tokenizer_map.insert(model_id, arc.clone());
    Ok(arc)
}

/// Estimate as length / 3.5, since 3 is reasonable estimate for code, and 4 for natural language
fn estimate_tokens(text: &str) -> usize {  1 + text.len() * 2 / 7 }

pub fn count_text_tokens_with_tokenizer(
    tokenizer: Arc<Tokenizer>,
    text: &str,
) -> Result<usize, String> {
    match tokenizer.encode_fast(text, false) {
        Ok(tokens) => Ok(tokens.len()),
        Err(e) => Err(format!("Encoding error: {e}")),
    }
}

pub fn count_text_tokens(text: &str) -> usize {
    estimate_tokens(text)
}
