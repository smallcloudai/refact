use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use crate::call_validation::SamplingParameters;
use serde_json::json;


pub async fn forward_to_openai_style_endpoint(
    bearer: Option<String>,
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    endpoint_template: &String,
    sampling_parameters: &SamplingParameters,
    // stream: bool,
) -> Result<serde_json::Value, String> {
    let url = endpoint_template.replace("$MODEL", model_name);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if let Some(t) = bearer {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(t.as_str()).unwrap());
    }
    let params_string = serde_json::to_string(sampling_parameters).unwrap();
    let mut params_json = serde_json::from_str::<serde_json::Value>(&params_string).unwrap();
    params_json["return_full_text"] = serde_json::Value::Bool(false);

    let data = json!({
        "model": model_name,
        "prompt": prompt,
        "echo": false,
        "stream": false,
        "temperature": sampling_parameters.temperature,
        "max_tokens": sampling_parameters.max_new_tokens,
    });
    let req = client.post(&url)
       .headers(headers)
       .body(data.to_string())
       .send()
       .await;
    let resp = req.map_err(|e| format!("when making request {}: {}", url, e))?;
    let status_code = resp.status().as_u16();
    let response_txt = resp.text().await.map_err(|e|
        format!("reading from socket {}: {}", url, e)
    )?;
    if status_code != 200 {
        return Err(format!("{} status={} text {}", url, status_code, response_txt));
    }
    Ok(serde_json::from_str(&response_txt).unwrap())
}

