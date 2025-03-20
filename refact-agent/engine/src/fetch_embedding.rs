use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tracing::error;

use crate::caps::EmbeddingModelRecord;
use crate::forward_to_hf_endpoint::get_embedding_hf_style;
use crate::forward_to_openai_endpoint::get_embedding_openai_style;

pub async fn get_embedding(
    client: Arc<AMutex<reqwest::Client>>,
    embedding_model: &EmbeddingModelRecord,
    text: Vec<String>,
) -> Result<Vec<Vec<f32>>, String> {
    match embedding_model.base.endpoint_style.to_lowercase().as_str() {
        "hf" => get_embedding_hf_style(client, text, embedding_model).await,
        "openai" => get_embedding_openai_style(client, text, embedding_model).await,
        _ => {
            error!("Invalid endpoint_embeddings_style: {}", embedding_model.base.endpoint_style);
            Err("Invalid endpoint_embeddings_style".to_string())
        }
    }
}

const SLEEP_ON_BIG_BATCH: u64 = 9000;
const SLEEP_ON_BATCH_ONE: u64 = 100;


// HF often returns 500 errors for no reason
pub async fn get_embedding_with_retries(
    client: Arc<AMutex<reqwest::Client>>,
    embedding_model: &EmbeddingModelRecord,
    text: Vec<String>,
    max_retries: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let mut attempt_n = 0;
    loop {
        attempt_n += 1;
        match get_embedding(
            client.clone(),
            embedding_model,
            text.clone(),
        ).await {
            Ok(embedding) => return Ok(embedding),
            Err(e) => {
                if attempt_n >= max_retries {
                    return Err(e);
                }
                if text.len() > 1 {
                    if e.contains("503") {
                        tracing::info!("normal sleep on 503");
                    } else {
                        tracing::warn!("will retry later, embedding model doesn't work: {}", e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(SLEEP_ON_BIG_BATCH)).await;
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_millis(SLEEP_ON_BATCH_ONE)).await;
                }
            }
        }
    }
}
