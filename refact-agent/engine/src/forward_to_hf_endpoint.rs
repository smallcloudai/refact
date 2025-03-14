use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_eventsource::EventSource;
use serde_json::json;
#[cfg(feature="vecdb")]
use tokio::sync::Mutex as AMutex;

use crate::call_validation::{ChatMeta, SamplingParameters};

// Idea: use USER_AGENT
// let user_agent = format!("{NAME}/{VERSION}; rust/unknown; ide/{ide:?}");


pub async fn forward_to_hf_style_endpoint(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    sampling_parameters: &SamplingParameters,
    meta: Option<ChatMeta>
) -> Result<serde_json::Value, String> {
    let url = endpoint_template.replace("$MODEL", model_name);
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    let params_string = serde_json::to_string(sampling_parameters).unwrap();
    let mut params_json = serde_json::from_str::<serde_json::Value>(&params_string).unwrap();
    params_json["return_full_text"] = serde_json::Value::Bool(false);

    let mut data = json!({
        "inputs": prompt,
        "parameters": params_json,
    });
    if let Some(meta) = meta {
        data["meta"] = serde_json::to_value(meta).unwrap();
    }
    
    let req = client.post(&url)
        .headers(headers)
        .body(data.to_string())
        .send()
        .await;
    let resp = req.map_err(|e| format!("{}", e))?;
    let status_code = resp.status().as_u16();
    let response_txt = resp.text().await.map_err(|e|
        format!("reading from socket {}: {}", url, e)
    )?;
    if status_code != 200 {
        return Err(format!("{} status={} text {}", url, status_code, response_txt));
    }
    Ok(match serde_json::from_str(&response_txt) {
        Ok(json) => json,
        Err(e) => return Err(format!("{}: {}", url, e)),
    })
}


pub async fn forward_to_hf_style_endpoint_streaming(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    sampling_parameters: &SamplingParameters,
    meta: Option<ChatMeta>
) -> Result<EventSource, String> {
    let url = endpoint_template.replace("$MODEL", model_name);
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    let params_string = serde_json::to_string(sampling_parameters).unwrap();
    let mut params_json = serde_json::from_str::<serde_json::Value>(&params_string).unwrap();
    params_json["return_full_text"] = serde_json::Value::Bool(false);

    let mut data = json!({
        "inputs": prompt,
        "parameters": params_json,
        "stream": true,
    });
    if let Some(meta) = meta {
        data["meta"] = serde_json::to_value(meta).unwrap();
    }

    let builder = client.post(&url)
        .headers(headers)
        .body(data.to_string());
    let event_source: EventSource = EventSource::new(builder).map_err(|e|
        format!("can't stream from {}: {}", url, e)
    )?;
    Ok(event_source)
}

#[cfg(feature="vecdb")]
#[derive(serde::Serialize)]
struct EmbeddingsPayloadHFOptions {
    pub wait_for_model: bool
}

#[cfg(feature="vecdb")]
impl EmbeddingsPayloadHFOptions {
    pub fn new() -> Self {
        Self { wait_for_model: true }
    }
}

#[cfg(feature="vecdb")]
#[derive(serde::Serialize)]
struct EmbeddingsPayloadHF {
    pub inputs: Vec<String>,
    pub options: EmbeddingsPayloadHFOptions,
}

#[cfg(feature="vecdb")]
pub async fn get_embedding_hf_style(
    client: std::sync::Arc<AMutex<reqwest::Client>>,
    text: Vec<String>,
    endpoint_template: &str,
    model_name: &str,
    api_key: &str,
) -> Result<Vec<Vec<f32>>, String> {
    let payload = EmbeddingsPayloadHF { inputs: text, options: EmbeddingsPayloadHFOptions::new() };
    let url = endpoint_template.to_string().replace("$MODEL", &model_name);

    let maybe_response = client.lock().await
        .post(&url)
        .bearer_auth(api_key.to_string())
        .json(&payload)
        .send()
        .await;

    match maybe_response {
        Ok(response) => {
            let status = response.status().clone();
            if status.is_success() {
                match response.json::<Vec<Vec<f32>>>().await {
                    Ok(embedding) =>
                        Ok(embedding),
                    Err(err) => Err(format!("Failed to parse the response: {:?}", err)),
                }
            } else {
                let body = response.text().await.unwrap().clone();
                if body.is_empty() {
                    Err(format!("Failed to get a response: {:?}", status))
                } else {
                    Err(format!("Failed to get a response: {:?}", body))
                }
            }
        }
        Err(err) => Err(format!("Failed to send a request: {:?}", err)),
    }
}
