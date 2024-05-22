use std::sync::Arc;

use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_eventsource::EventSource;
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::call_validation;
use crate::call_validation::SamplingParameters;

pub async fn forward_to_openai_style_endpoint(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    endpoint_chat_passthrough: &String,
    sampling_parameters: &SamplingParameters,
    tools_mb: Option<Vec<serde_json::Value>>
) -> Result<serde_json::Value, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let url = if !is_passthrough { endpoint_template.replace("$MODEL", model_name) } else { endpoint_chat_passthrough.clone() };
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    info!("TEMP {}", sampling_parameters.temperature.unwrap());
    let mut data = json!({
        "model": model_name,
        "stream": false,
        "temperature": sampling_parameters.temperature,
        "max_tokens": sampling_parameters.max_new_tokens,
        "stop": sampling_parameters.stop,
    });
    if let Some(tools) = tools_mb {
        data["tools"] = serde_json::Value::Array(tools);
    }
    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
        data["echo"] = serde_json::Value::Bool(false);
    }
    // When cancelling requests, coroutine ususally gets aborted here on the following line.
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
    // 400 "client error" is likely a json that we rather accept here, pick up error details as we analyse json fields at the level
    // higher, the most often 400 is no such model.
    if status_code != 200 && status_code != 400 {
        return Err(format!("{} status={} text {}", url, status_code, response_txt));
    }
    if status_code != 200 {
        info!("forward_to_openai_style_endpoint: {} {}\n{}", url, status_code, response_txt);
    }
    Ok(serde_json::from_str(&response_txt).unwrap())
}

pub async fn forward_to_openai_style_endpoint_streaming(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    endpoint_chat_passthrough: &String,
    sampling_parameters: &SamplingParameters,
    tools_mb: Option<Vec<serde_json::Value>>
) -> Result<EventSource, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let url = if !is_passthrough { endpoint_template.replace("$MODEL", model_name) } else { endpoint_chat_passthrough.clone() };
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    let mut data = json!({
        "model": model_name,
        "stream": true,
        "temperature": sampling_parameters.temperature,
        "max_tokens": sampling_parameters.max_new_tokens,
    });
    if let Some(tools) = tools_mb {
        data["tools"] = serde_json::Value::Array(tools);
    }
    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
    }
    let builder = client.post(&url)
        .headers(headers)
        .body(data.to_string());
    let event_source: EventSource = EventSource::new(builder).map_err(|e|
        format!("can't stream from {}: {}", url, e)
    )?;
    Ok(event_source)
}

fn passthrough_messages_to_json(
    data: &mut serde_json::Value,
    prompt: &str,
) {
    assert!(prompt.starts_with("PASSTHROUGH "));
    let messages_str = &prompt[12..];
    let messages: Vec<call_validation::ChatMessage> = serde_json::from_str(&messages_str).unwrap();
    data["messages"] = serde_json::json!(messages);
}

#[derive(Serialize)]
struct EmbeddingsPayloadOpenAI {
    pub input: Vec<String>,
    pub model: String,
}

#[derive(Deserialize)]
struct EmbeddingsResultOpenAI {
    pub embedding: Vec<f32>,
    pub index: usize,
}

pub async fn get_embedding_openai_style(
    client: Arc<AMutex<reqwest::Client>>,
    text: Vec<String>,
    endpoint_template: &String,
    model_name: &String,
    api_key: &String,
) -> Result<Vec<Vec<f32>>, String> {
    #[allow(non_snake_case)]
    let B = text.len();
    let payload = EmbeddingsPayloadOpenAI {
        input: text,
        model: model_name.clone(),
    };
    let url = endpoint_template.clone();
    let api_key_clone = api_key.clone();
    let response = client.lock().await
        .post(&url)
        .bearer_auth(api_key_clone.clone())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send a request: {:?}", e))?;

    if !response.status().is_success() {
        info!("get_embedding_openai_style: {:?}", response);
        return Err(format!("get_embedding_openai_style: bad status: {:?}", response.status()));
    }

    let json = response.json::<serde_json::Value>()
        .await
        .map_err(|err| format!("get_embedding_openai_style: failed to parse the response: {:?}", err))?;

    // info!("get_embedding_openai_style: {:?}", json);
    // {"data":[{"embedding":[0.0121664945...],"index":0,"object":"embedding"}, {}, {}]}
    let unordered: Vec<EmbeddingsResultOpenAI> = match serde_json::from_value(json["data"].clone()) {
        Ok(x) => x,
        Err(err) => {
            return Err(format!("get_embedding_openai_style: failed to parse unordered: {:?}", err));
        }
    };
    let mut result: Vec<Vec<f32>> = vec![vec![]; B];
    for ures in unordered.into_iter() {
        let index = ures.index;
        if index >= B {
            return Err(format!("get_embedding_openai_style: index out of bounds: {:?}", json));
        }
        result[index] = ures.embedding;
    }
    Ok(result)
}
