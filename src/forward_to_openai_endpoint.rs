use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_eventsource::EventSource;
use serde_json::json;
use crate::call_validation;
use crate::call_validation::SamplingParameters;
use tracing::info;


pub async fn forward_to_openai_style_endpoint(
    save_url: &mut String,
    bearer: String,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    endpoint_chat_passthrough: &String,
    sampling_parameters: &SamplingParameters,
) -> Result<serde_json::Value, String> {
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
        "echo": false,
        "stream": false,
        "temperature": sampling_parameters.temperature,
        "max_tokens": sampling_parameters.max_new_tokens,
    });
    if is_passthrough {
        _passthrough_messages_to_json(&mut data, prompt);
    } else {
        data["prompt"] = serde_json::Value::String(prompt.to_string());
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
    // info!("forward_to_openai_style_endpoint: {} {}\n{}", url, status_code, response_txt);
    if status_code != 200 {
        return Err(format!("{} status={} text {}", url, status_code, response_txt));
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
    if is_passthrough {
        _passthrough_messages_to_json(&mut data, prompt);
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

fn _passthrough_messages_to_json(
    data: &mut serde_json::Value,
    prompt: &str,
) {
    assert!(prompt.starts_with("PASSTHROUGH "));
    let messages_str = &prompt[12..];
    let messages: Vec<call_validation::ChatMessage> = serde_json::from_str(&messages_str).unwrap();
    data["messages"] = serde_json::json!(messages);
}
