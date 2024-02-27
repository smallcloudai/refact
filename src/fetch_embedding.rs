use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tracing::error;

use crate::forward_to_hf_endpoint::get_embedding_hf_style;
use crate::forward_to_openai_endpoint::get_embedding_openai_style;

pub async fn get_embedding(
    client: Arc<AMutex<reqwest::Client>>,
    endpoint_embeddings_style: &String,
    model_name: &String,
    endpoint_template: &String,
    text: String,
    api_key: &String,
) -> Result<Vec<f32>, String> {
    match endpoint_embeddings_style.to_lowercase().as_str() {
        "hf" => get_embedding_hf_style(client, text, endpoint_template, model_name, api_key).await,
        "openai" => get_embedding_openai_style(client, text, endpoint_template, model_name, api_key).await,
        _ => {
            error!("Invalid endpoint_embeddings_style: {}", endpoint_embeddings_style);
            Err("Invalid endpoint_embeddings_style".to_string())
        }
    }
}


// HF often returns 500 errors for no reason
pub async fn try_get_embedding(
    client: Arc<AMutex<reqwest::Client>>,
    endpoint_embeddings_style: &String,
    model_name: &String,
    endpoint_template: &String,
    text: String,
    api_key: &String,
    max_retries: usize,
) -> Result<Vec<f32>, String> {
    let sleep_on_failure_ms = 300;
    let mut retries = 0;
    loop {
        retries += 1;
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
                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_on_failure_ms)).await;
                if retries > max_retries {
                    return Err(e);
                }
            }
        }
    }
}
