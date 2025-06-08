use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::USER_AGENT;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_eventsource::EventSource;
use serde_json::json;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::call_validation::{ChatMeta, SamplingParameters};
use crate::caps::BaseModelRecord;
use crate::custom_error::MapErrToString;
use crate::scratchpads::chat_utils_limit_history::CompressionStrength;
use crate::caps::EmbeddingModelRecord;

pub async fn forward_to_openai_style_endpoint(
    model_rec: &BaseModelRecord,
    prompt: &str,
    client: &reqwest::Client,
    sampling_parameters: &SamplingParameters,
    meta: Option<ChatMeta>
) -> Result<serde_json::Value, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !model_rec.api_key.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", model_rec.api_key)).unwrap());
    }
    if model_rec.support_metadata {
        headers.insert(USER_AGENT, HeaderValue::from_str(&format!("refact-lsp {}", crate::version::build::PKG_VERSION)).unwrap());
    }
    let mut data = json!({
        "model": model_rec.name.clone(),
        "stream": false,
    });
    if !sampling_parameters.stop.is_empty() {  // openai does not like empty stop
        data["stop"] = serde_json::Value::from(sampling_parameters.stop.clone());
    };
    if let Some(n) = sampling_parameters.n {
        data["n"] = serde_json::Value::from(n);
    }
    if let Some(reasoning_effort) = sampling_parameters.reasoning_effort.clone() {
        data["reasoning_effort"] = serde_json::Value::String(reasoning_effort.to_string());
    } else if let Some(thinking) = sampling_parameters.thinking.clone() {
        data["thinking"] = thinking.clone();
    } else if let Some(enable_thinking) = sampling_parameters.enable_thinking {
        data["enable_thinking"] = serde_json::Value::Bool(enable_thinking);
        data["temperature"] = serde_json::Value::from(sampling_parameters.temperature);
    } else if let Some(temperature) = sampling_parameters.temperature {
        data["temperature"] = serde_json::Value::from(temperature);
    }
    data["max_completion_tokens"] = serde_json::Value::from(sampling_parameters.max_new_tokens);
    info!("Request: model={}, reasoning_effort={}, T={}, n={}, stream=false", 
        model_rec.name,
        sampling_parameters.reasoning_effort.clone().map(|x| x.to_string()).unwrap_or("none".to_string()),
        sampling_parameters.temperature.clone().map(|x| x.to_string()).unwrap_or("none".to_string()),
        sampling_parameters.n.clone().map(|x| x.to_string()).unwrap_or("none".to_string())
    );
    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt, &model_rec.name);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
        data["echo"] = serde_json::Value::Bool(false);
    }
    if let Some(meta) = meta {
        data["meta"] = json!(meta);
    }

    // When cancelling requests, coroutine ususally gets aborted here on the following line.
    let req = client.post(&model_rec.endpoint)
        .headers(headers)
        .body(data.to_string())
        .send()
        .await;
    let resp = req.map_err_to_string()?;
    let status_code = resp.status().as_u16();
    let response_txt = resp.text().await.map_err(|e|
        format!("reading from socket {}: {}", model_rec.endpoint, e)
    )?;
    // 400 "client error" is likely a json that we rather accept here, pick up error details as we analyse json fields at the level
    // higher, the most often 400 is no such model.
    if status_code != 200 && status_code != 400 {
        return Err(format!("{} status={} text {}", model_rec.endpoint, status_code, response_txt));
    }
    if status_code != 200 {
        tracing::info!("forward_to_openai_style_endpoint: {} {}\n{}", model_rec.endpoint, status_code, response_txt);
    }
    let parsed_json: serde_json::Value = match serde_json::from_str(&response_txt) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to parse JSON response: {}\n{}", e, response_txt)),
    };
    Ok(parsed_json)
}

pub async fn forward_to_openai_style_endpoint_streaming(
    model_rec: &BaseModelRecord,
    prompt: &str,
    client: &reqwest::Client,
    sampling_parameters: &SamplingParameters,
    meta: Option<ChatMeta>
) -> Result<EventSource, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !model_rec.api_key.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", model_rec.api_key)).unwrap());
    }
    if model_rec.support_metadata {
        headers.insert(USER_AGENT, HeaderValue::from_str(format!("refact-lsp {}", crate::version::build::PKG_VERSION).as_str()).unwrap());
    }

    let mut data = json!({
        "model": model_rec.name,
        "stream": true,
        "stream_options": {"include_usage": true},
    });

    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt, &model_rec.name);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
    }

    if !sampling_parameters.stop.is_empty() {  // openai does not like empty stop
        data["stop"] = serde_json::Value::from(sampling_parameters.stop.clone());
    };
    if let Some(n) = sampling_parameters.n{
        data["n"] = serde_json::Value::from(n);
    }

    if let Some(reasoning_effort) = sampling_parameters.reasoning_effort.clone() {
        data["reasoning_effort"] = serde_json::Value::String(reasoning_effort.to_string());
    } else if let Some(thinking) = sampling_parameters.thinking.clone() {
        data["thinking"] = thinking.clone();
    } else if let Some(enable_thinking) = sampling_parameters.enable_thinking {
        data["enable_thinking"] = serde_json::Value::Bool(enable_thinking);
        data["temperature"] = serde_json::Value::from(sampling_parameters.temperature);
    }else if let Some(temperature) = sampling_parameters.temperature {
        data["temperature"] = serde_json::Value::from(temperature);
    }
    data["max_completion_tokens"] = serde_json::Value::from(sampling_parameters.max_new_tokens);

    info!("Request: model={}, reasoning_effort={}, T={}, n={}, stream=true", 
        model_rec.name,
        sampling_parameters.reasoning_effort.clone().map(|x| x.to_string()).unwrap_or("none".to_string()),
        sampling_parameters.temperature.clone().map(|x| x.to_string()).unwrap_or("none".to_string()),
        sampling_parameters.n.clone().map(|x| x.to_string()).unwrap_or("none".to_string())
    );

    if let Some(meta) = meta {
        data["meta"] = json!(meta);
    }

    if model_rec.endpoint.is_empty() {
        return Err(format!("No endpoint configured for {}", model_rec.id));
    }
    let builder = client.post(&model_rec.endpoint)
        .headers(headers)
        .body(data.to_string());
    let event_source: EventSource = EventSource::new(builder).map_err(|e|
        format!("can't stream from {}: {}", model_rec.endpoint, e)
    )?;
    Ok(event_source)
}

// NOTE: questionable function, no idea why we need it
fn passthrough_messages_to_json(
    data: &mut serde_json::Value,
    prompt: &str,
    model_name: &str,
) {
    assert!(prompt.starts_with("PASSTHROUGH "));
    let messages_str = &prompt[12..];
    let big_json: serde_json::Value = serde_json::from_str(&messages_str).unwrap();

    data["messages"] = big_json["messages"].clone();
    if let Some(tools) = big_json.get("tools") {
        if model_name != "o1-mini" {
            data["tools"] = tools.clone();
        }
    }
}

pub fn try_get_compression_from_prompt(
    prompt: &str,
) -> serde_json::Value {
    let big_json: serde_json::Value = if prompt.starts_with("PASSTHROUGH ") {
        serde_json::from_str( &prompt[12..]).unwrap()
    } else {
        return json!(CompressionStrength::Absent);
    };
    if let Some(compression_strength) = big_json.get("compression_strength") {
        compression_strength.clone()
    } else {
        json!(CompressionStrength::Absent)
    }
}

#[derive(serde::Serialize)]
struct EmbeddingsPayloadOpenAI {
    pub input: Vec<String>,
    pub model: String,
}

#[derive(serde::Deserialize)]
struct EmbeddingsResultOpenAI {
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(serde::Deserialize)]
struct EmbeddingsResultOpenAINoIndex {
    pub embedding: Vec<f32>,
}

pub async fn get_embedding_openai_style(
    client: std::sync::Arc<AMutex<reqwest::Client>>,
    text: Vec<String>,
    model_rec: &EmbeddingModelRecord,
) -> Result<Vec<Vec<f32>>, String> {
    if model_rec.base.endpoint.is_empty() {
        return Err(format!("No embedding endpoint configured"));
    }
    if model_rec.base.api_key.is_empty() {
        return Err(format!("Cannot access embedding model, because api_key is empty"));
    }
    #[allow(non_snake_case)]
    let B: usize = text.len();
    let payload = EmbeddingsPayloadOpenAI {
        input: text,
        model: model_rec.base.name.to_string(),
    };
    let response = client.lock().await
        .post(&model_rec.base.endpoint)
        .bearer_auth(&model_rec.base.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send a request: {:?}", e))?;

    if !response.status().is_success() {
        if response.status().as_u16() != 503 {
            info!("get_embedding_openai_style: {:?}", response);
        }
        return Err(format!("get_embedding_openai_style: bad status: {:?}", response.status()));
    }

    let json = response.json::<serde_json::Value>()
        .await
        .map_err(|err| format!("get_embedding_openai_style: failed to parse the response: {:?}", err))?;

    // info!("get_embedding_openai_style: {:?}", json);
    // {"data":[{"embedding":[0.0121664945...],"index":0,"object":"embedding"}, {}, {}]}
    // or {"data":[{"embedding":[0.0121664945...]}, {}, {}]} without index

    let mut result: Vec<Vec<f32>> = vec![vec![]; B];
    match serde_json::from_value::<Vec<EmbeddingsResultOpenAI>>(json["data"].clone()) {
        Ok(unordered) => {
            for ures in unordered.into_iter() {
                let index = ures.index;
                if index >= B {
                    return Err(format!("get_embedding_openai_style: index out of bounds: {:?}", json));
                }
                result[index] = ures.embedding;
            }
        },
        Err(_) => {
            match serde_json::from_value::<Vec<EmbeddingsResultOpenAINoIndex>>(json["data"].clone()) {
                Ok(ordered) => {
                    if ordered.len() != B {
                        return Err(format!("get_embedding_openai_style: response length mismatch: expected {}, got {}",
                                          B, ordered.len()));
                    }
                    for (i, res) in ordered.into_iter().enumerate() {
                        result[i] = res.embedding;
                    }
                },
                Err(err) => {
                    tracing::info!("get_embedding_openai_style: failed to parse response: {:?}, {:?}", err, json);
                    return Err(format!("get_embedding_openai_style: failed to parse response: {:?}", err));
                }
            }
        }
    }
    Ok(result)
}
