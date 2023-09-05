use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde_json::json;


pub async fn simple_forward_to_hf_endpoint_no_streaming(
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    bearer: Option<String>,
    // sampling_parameters: &Dict<String, Any>,
    // stream: bool,
    // auth_from_client:
    // Result<serde_json::Value, serde_json::Error> Option<&str>,
) -> Result<serde_json::Value, serde_json::Error> {
    let url = format!("https://api-inference.huggingface.co/models/{}", model_name);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    if let Some(t) = bearer {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", t)).unwrap());
    }
    let data = json!({
        "inputs": prompt,
        "parameters": {
            "return_full_text": false,
        },
        // "stream": stream,
    });
    let response = client.post(&url)
       .headers(headers)
       .body(data.to_string())
       .send()
       .await;
    let response_txt = response.unwrap().text().await.unwrap();
    // let response_json = serde_json::from_str(&response_txt).unwrap();
    // let t3 = std::time::Instant::now();
    // // println!("forward_to_hf_endpoint: http status {}, response text was:\n{}", response.unwrap().status(), response_txt);
    // return response_json.into_iter().map(move |line| {
    //     // println!("-"*20, "line", "-"*20, "%0.2fms" % ((t1 - t0) * 1000));
    //     // println!("{}", line);
    //     // println!("-"*20, "/line", "-"*20);
    //     line
    // });
    Ok(serde_json::from_str(&response_txt).unwrap())
}


// with streaming:
// use futures::stream::Stream;
// -> impl Stream<Item = String>

