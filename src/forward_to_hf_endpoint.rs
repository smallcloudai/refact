use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde_json::json;


// async def real_work(
//     model_name: str,
//     prompt: str,
//     sampling_parameters: Dict[str, Any],
//     stream: bool,
//     auth_from_client: Optional[str],
// ):
//     session = global_hf_session_get()
//     url = "https://api-inference.huggingface.co/models/" + model_name
//     headers = {
//         "Authorization": "Bearer " + (auth_from_client or os.environ["HUGGINGFACE_TOKEN"]),
//     }
//     data = {
//         "inputs": prompt,
//         "parameters": sampling_parameters,
//         "stream": stream,
//     }
//     t0 = time.time()
//     if stream:
//         async with session.post(url, json=data, headers=headers) as response:
//             async for byteline in response.content:
//                 # TODO: handle response errors
//                 txt = byteline.decode("utf-8").strip()
//                 if not txt.startswith("data:"):
//                     continue
//                 txt = txt[5:]
//                 # print("-"*20, "line", "-"*20, "%0.2fms" % ((time.time() - t0) * 1000))
//                 # print(txt)
//                 # print("-"*20, "/line", "-"*20)
//                 line = json.loads(txt)
//                 yield line
//     else:
//         async with session.post(url, json=data, headers=headers) as response:
//             response_txt = await response.text()
//             if response.status == 200:
//                 response_json = json.loads(response_txt)
//                 yield response_json
//             else:
//                 logger.warning("forward_to_hf_endpoint: http status %s, response text was:\n%s" % (response.status, response_txt))
//                 raise ValueError(json.dumps({"error": "hf_endpoint says: %s" % (textwrap.shorten(response_txt, 50))}))


pub async fn simple_forward_to_hf_endpoint_no_streaming(
    model_name: &str,
    prompt: &str,
    client: &reqwest::Client,
    hf_api_token: &str,
    // sampling_parameters: &Dict<String, Any>,
    // stream: bool,
    // auth_from_client:
    // Result<serde_json::Value, serde_json::Error> Option<&str>,
) -> Result<serde_json::Value, serde_json::Error> {
    let url = format!("https://api-inference.huggingface.co/models/{}", model_name);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", hf_api_token)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
    let data = json!({
        "inputs": prompt,
        // "parameters": sampling_parameters,
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

