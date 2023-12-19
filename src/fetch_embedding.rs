use reqwest;
use serde::Serialize;
use tracing::error;

use crate::forward_to_hf_endpoint::get_embedding_hf_style;
use crate::forward_to_openai_endpoint::get_embedding_openai_style;


pub async fn get_embedding(
    endpoint_embeddings_style: &String,
    model_name: &String,
    endpoint_template: &String,
    text: String,
    api_key: &String,
) -> Result<Vec<f32>, String> {
    match endpoint_embeddings_style.to_lowercase().as_str() {
        "hf" => get_embedding_hf_style(text, endpoint_template, model_name, api_key).await,
        "openai" => get_embedding_openai_style(text, endpoint_template, model_name, api_key).await,
        _ => {
            error!("Invalid endpoint_embeddings_style: {}", endpoint_embeddings_style);
            Err("Invalid endpoint_embeddings_style".to_string())
        }
    }
}


// HF often returns 500 errors for no reason
pub async fn try_get_embedding(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_request() {
        let _m = mockito::mock("POST", "/models/valid_model")
            .with_status(200)
            .with_body(r#"{"embedding": [1.0, 2.0, 3.0]}"#)
            .create();

        let text = "sample text".to_string();
        let model_name = "valid_model".to_string();
        let api_key = "valid_api_key".to_string();

        let result = get_embedding(text, &model_name, api_key).await.unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn test_invalid_api_key() {
        let _m = mockito::mock("POST", "/models/valid_model")
            .with_status(401)
            .create();

        let text = "sample text".to_string();
        let model_name = "valid_model".to_string();
        let api_key = "invalid_api_key".to_string();

        let result = get_embedding(text, &model_name, api_key).await.unwrap();

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let mock = mockito::mock("POST", "/models/valid_model")
            .with_status(200)
            .with_body(r#"{"embedding": [1.0, 2.0, 3.0]}"#)
            .expect(10)  // Expect 10 calls
            .create();

        let handles: Vec<_> = (0..10).map(|_| {
            let text = "sample text".to_string();
            let model_name = "valid_model".to_string();
            let api_key = "valid_api_key".to_string();

            get_embedding(text, &model_name, api_key)
        }).collect();

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), vec![1.0, 2.0, 3.0]);
        }

        mock.assert();
    }

    #[tokio::test]
    async fn test_empty_text_input() {
        let _m = mockito::mock("POST", "/models/valid_model")
            .with_status(200)
            .with_body(r#"{"embedding": []}"#)
            .create();

        let text = "".to_string();
        let model_name = "valid_model".to_string();
        let api_key = "valid_api_key".to_string();

        let result = get_embedding(text, &model_name, api_key).await.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<f32>::new());
    }

    #[tokio::test]
    async fn test_invalid_model_name() {
        let _m = mockito::mock("POST", "/models/invalid_model")
            .with_status(404)
            .create();

        let text = "sample text".to_string();
        let model_name = "invalid_model".to_string();
        let api_key = "valid_api_key".to_string();

        let result = get_embedding(text, &model_name, api_key).await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_network_failure() {
        let _m = mockito::mock("POST", "/models/valid_model")
            .with_status(500) // Internal Server Error to simulate server-side failure
            .create();

        let text = "sample text".to_string();
        let model_name = "valid_model".to_string();
        let api_key = "valid_api_key".to_string();

        let result = get_embedding(text, &model_name, api_key).await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_embeddings() {
        let mock1 = mockito::mock("POST", "/models/model1")
            .with_status(200)
            .with_body(r#"{"embedding": [1.0, 2.0]}"#)
            .create();

        let mock2 = mockito::mock("POST", "/models/model2")
            .with_status(200)
            .with_body(r#"{"embedding": [3.0, 4.0]}"#)
            .create();

        let text = "sample text".to_string();
        let model_names = vec!["model1", "model2"];
        let api_key = "valid_api_key".to_string();

        for model_name in model_names {
            let result = get_embedding(text.clone(), &model_name.to_string(), api_key.clone()).await.unwrap();
            assert!(result.is_ok());
        }

        mock1.assert();
        mock2.assert();
    }
}