use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tracing::error;

use crate::forward_to_hf_endpoint::get_embedding_hf_style;
use crate::forward_to_openai_endpoint::get_embedding_openai_style;

pub async fn get_embedding(
    client: Arc<AMutex<reqwest::Client>>,
    endpoint_embeddings_style: &str,
    model_name: &str,
    endpoint_template: &str,
    text: Vec<String>,
    api_key: &str,
) -> Result<Vec<Vec<f32>>, String> {
    match endpoint_embeddings_style.to_lowercase().as_str() {
        "hf" => get_embedding_hf_style(client, text, endpoint_template, model_name, api_key).await,
        "openai" => get_embedding_openai_style(client, text, endpoint_template, model_name, api_key).await,
        _ => {
            error!("Invalid endpoint_embeddings_style: {}", endpoint_embeddings_style);
            Err("Invalid endpoint_embeddings_style".to_string())
        }
    }
}

const SLEEP_ON_BIG_BATCH: u64 = 9000;
const SLEEP_ON_BATCH_ONE: u64 = 100;


// HF often returns 500 errors for no reason
pub async fn get_embedding_with_retry(
    client: Arc<AMutex<reqwest::Client>>,
    endpoint_embeddings_style: &str,
    model_name: &str,
    endpoint_template: &str,
    text: Vec<String>,
    api_key: &str,
    max_retries: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let mut attempt_n = 0;
    loop {
        attempt_n += 1;
        match get_embedding(
            client.clone(),
            endpoint_embeddings_style,
            model_name,
            endpoint_template,
            text.clone(),
            api_key,
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
