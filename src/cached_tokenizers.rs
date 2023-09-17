use reqwest::header::AUTHORIZATION;
use tracing::info;
use tokio::io::AsyncWriteExt;
use std::path::Path;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub async fn download_tokenizer_file(
    http_client: &reqwest::Client,
    http_path: &str,
    api_token: Option<String>,
    to: impl AsRef<Path>,
) -> Result<(), String> {
    if to.as_ref().exists() {
        return Ok(());
    }
    info!("downloading tokenizer \"{}\" to {}...", http_path, to.as_ref().display());
    tokio::fs::create_dir_all(
            to.as_ref().parent().ok_or_else(|| "tokenizer path has no parent")?,
        )
        .await
        .map_err(|e| format!("failed to create parent dir: {}", e))?;
    let mut req = http_client.get(http_path);
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
