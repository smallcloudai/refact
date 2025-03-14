use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::USER_AGENT;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_eventsource::EventSource;
use serde_json::json;
#[cfg(feature="vecdb")]
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::call_validation::{ChatMeta, SamplingParameters};
use crate::scratchpads::chat_utils_limit_history::CompressionStrength;

pub async fn forward_to_openai_style_endpoint(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    endpoint_chat_passthrough: &String,
    sampling_parameters: &SamplingParameters,
    is_metadata_supported: bool,
    meta: Option<ChatMeta>
) -> Result<serde_json::Value, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let url = if !is_passthrough { endpoint_template.replace("$MODEL", model_name) } else { endpoint_chat_passthrough.clone() };
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    if is_metadata_supported {
        headers.insert(USER_AGENT, HeaderValue::from_str(format!("refact-lsp {}", crate::version::build_info::PKG_VERSION).as_str()).unwrap());
    }
    let mut data = json!({
        "model": model_name,
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
    } else {
        data["temperature"] = serde_json::Value::from(sampling_parameters.temperature);
    }
    data["max_completion_tokens"] = serde_json::Value::from(sampling_parameters.max_new_tokens);
    info!("NOT STREAMING TEMP {}", sampling_parameters.temperature
        .map(|x| x.to_string())
        .unwrap_or("None".to_string()));
    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt, model_name);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
        data["echo"] = serde_json::Value::Bool(false);
    }
    if let Some(meta) = meta {
        data["meta"] = json!(meta);
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
    let parsed_json: serde_json::Value = match serde_json::from_str(&response_txt) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to parse JSON response: {}\n{}", e, response_txt)),
    };
    Ok(parsed_json)
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
    is_metadata_supported: bool,
    meta: Option<ChatMeta>
) -> Result<EventSource, String> {
    let is_passthrough = prompt.starts_with("PASSTHROUGH ");
    let url = if !is_passthrough { endpoint_template.replace("$MODEL", model_name) } else { endpoint_chat_passthrough.clone() };
    save_url.clone_from(&&url);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if !bearer.is_empty() {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(format!("Bearer {}", bearer).as_str()).unwrap());
    }
    if is_metadata_supported {
        headers.insert(USER_AGENT, HeaderValue::from_str(format!("refact-lsp {}", crate::version::build_info::PKG_VERSION).as_str()).unwrap());
    }

    let mut data = json!({
        "model": model_name,
        "stream": true,
        "stream_options": {"include_usage": true},
    });

    if is_passthrough {
        passthrough_messages_to_json(&mut data, prompt, model_name);
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
    } else {
        data["temperature"] = serde_json::Value::from(sampling_parameters.temperature);
    }
    data["max_completion_tokens"] = serde_json::Value::from(sampling_parameters.max_new_tokens);

    info!("STREAMING TEMP {}", sampling_parameters.temperature
        .map(|x| x.to_string())
        .unwrap_or("None".to_string()));

    if let Some(meta) = meta {
        data["meta"] = json!(meta);
    }
    let builder = client.post(&url)
        .headers(headers)
        .body(data.to_string());
    let event_source: EventSource = EventSource::new(builder).map_err(|e|
        format!("can't stream from {}: {}", url, e)
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

#[cfg(feature="vecdb")]
#[derive(serde::Serialize)]
struct EmbeddingsPayloadOpenAI {
    pub input: Vec<String>,
    pub model: String,
}

#[cfg(feature="vecdb")]
#[derive(serde::Deserialize)]
struct EmbeddingsResultOpenAI {
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[cfg(feature="vecdb")]
pub async fn get_embedding_openai_style(
    client: std::sync::Arc<AMutex<reqwest::Client>>,
    text: Vec<String>,
    endpoint_template: &str,
    model_name: &str,
    api_key: &str,
) -> Result<Vec<Vec<f32>>, String> {
    if endpoint_template.is_empty() {
        return Err(format!("no embedding_endpoint configured"));
    }
    if api_key.is_empty() {
        return Err(format!("cannot access embedding model, because api_key is empty"));
    }
    #[allow(non_snake_case)]
    let B = text.len();
    let payload = EmbeddingsPayloadOpenAI {
        input: text,
        model: model_name.to_string(),
    };
    let url = endpoint_template.to_string();
    let api_key_clone = api_key.to_string();
    let response = client.lock().await
        .post(url)
        .bearer_auth(api_key_clone.to_string())
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
