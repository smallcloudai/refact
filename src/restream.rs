use tracing::{error, info};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use hyper::{Body, Response, StatusCode};
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::forward_to_hf_endpoint;
use crate::forward_to_openai_endpoint;

use reqwest_eventsource::Event;
use futures::StreamExt;
use async_stream::stream;
use serde_json::json;

use crate::call_validation::SamplingParameters;
use crate::caps::CodeAssistantCaps;


pub fn explain_whats_wrong(status_code: StatusCode, msg: String) -> Response<Body> {
    let body = json!({"detail": msg}).to_string();
    error!("client will see {}", body);
    let response = Response::builder()
       .status(status_code)
       .header("Content-Type", "application/json")
       .body(Body::from(body))
       .unwrap();
    response
}

pub async fn scratchpad_interaction_not_stream(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    mut scratchpad: Box<dyn ScratchpadAbstract>,
    prompt: &str,
    model_name: String,
    client: reqwest::Client,
    bearer: Option<String>,
    parameters: &SamplingParameters,
) -> Result<Response<Body>, Response<Body>> {
    let t2 = std::time::SystemTime::now();
    let (endpoint_style, endpoint_template) = {
        let caps_locked = caps.read().unwrap();
        (caps_locked.endpoint_style.clone(), caps_locked.endpoint_template.clone())
    };
    let model_says = if endpoint_style == "hf" {
        forward_to_hf_endpoint::forward_to_hf_style_endpoint(
            bearer.clone(),
            &model_name,
            &prompt,
            &client,
            &endpoint_template,
            &parameters,
        ).await
    } else {
        forward_to_openai_endpoint::forward_to_openai_style_endpoint(
            bearer.clone(),
            &model_name,
            &prompt,
            &client,
            &endpoint_template,
            &parameters,
        ).await
    }.map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("forward_to_endpoint: {}", e))
    )?;
    info!("forward to endpoint {:?}", t2.elapsed());

    let scratchpad_result: Result<serde_json::Value, String>;
    if let Some(hf_arr) = model_says.as_array() {
        let choices = hf_arr.iter()
            .map(|x| {
                x.get("generated_text").unwrap().as_str().unwrap().to_string()
            }).collect::<Vec<_>>();
        let stopped = vec![false; choices.len()];
        scratchpad_result = scratchpad.response_n_choices(choices, stopped);

    } else if let Some(oai_choices) = model_says.get("choices") {
        let choices = oai_choices.as_array().unwrap().iter()
            .map(|x| {
                x.get("text").unwrap().as_str().unwrap().to_string()
            }).collect::<Vec<_>>();
        let stopped = oai_choices.as_array().unwrap().iter()
            .map(|x| {
                x.get("finish_reason").unwrap_or(&json!("")).as_str().unwrap().to_string().starts_with("stop")
            }).collect::<Vec<_>>();
        scratchpad_result = scratchpad.response_n_choices(choices, stopped);

    } else if let Some(err) = model_says.get("error") {
        return Ok(explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,
            format!("model says: {}", err)
        ));

    } else if let Some(msg) = model_says.get("human_readable_message") {
        return Ok(explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,
            format!("model says: {}", msg)
        ));

    } else {
        return Ok(explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,
            format!("unrecognized response: {:?}", model_says))
        );
    }

    if let Err(scratchpad_result_str) = scratchpad_result {
        return Ok(explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,
            format!("scratchpad: {}", scratchpad_result_str))
        );
    }
    let mut scratchpad_response_json = scratchpad_result.unwrap();
    scratchpad_response_json["created"] = json!(t2.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64 / 1000.0);

    let txt = serde_json::to_string_pretty(&scratchpad_response_json).unwrap();
    info!("handle_v1_code_completion return {}", txt);
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(txt))
        .unwrap();
    return Ok(response);
}

pub async fn scratchpad_interaction_stream(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    mut scratchpad: Box<dyn ScratchpadAbstract>,
    prompt: &str,
    mut model_name: String,
    client: reqwest::Client,
    bearer: Option<String>,
    parameters: &SamplingParameters,
) -> Result<Response<Body>, Response<Body>> {
    let t1 = std::time::SystemTime::now();
    let (endpoint_style, endpoint_template) = {
        let caps_locked = caps.read().unwrap();
        (caps_locked.endpoint_style.clone(), caps_locked.endpoint_template.clone())
    };
    let mut event_source = if endpoint_style == "hf" {
        forward_to_hf_endpoint::forward_to_hf_style_endpoint_streaming(
            bearer.clone(),
            &model_name,
            &prompt,
            &client,
            &endpoint_template,
            &parameters,
        ).await
    } else {
        forward_to_openai_endpoint::forward_to_openai_style_endpoint_streaming(
            bearer.clone(),
            &model_name,
            &prompt,
            &client,
            &endpoint_template,
            &parameters,
        ).await
    }.map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("forward_to_endpoint: {}", e))
    )?;

    let evstream = stream! {
        let scratch = &mut scratchpad;
        let mut finished: bool = false;
        let mut problem_str: String = String::new();
        let mut was_correct_output_even_if_error = false;
        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => {},
                Ok(Event::Message(message)) => {
                    info!("Message: {:#?}", message);
                    if message.data.starts_with("[DONE]") {
                        break;
                    }
                    let json = serde_json::from_str::<serde_json::Value>(&message.data).unwrap();
                    let value_str;
                    if let Some(token) = json.get("token") { // hf style produces this
                        let text = token.get("text").unwrap().as_str().unwrap().to_string();
                        let mut value: serde_json::Value;
                        (value, finished) = scratch.response_streaming(text, false).unwrap();
                        value["created"] = json!(t1.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64 / 1000.0);
                        value["model"] = json!(model_name.clone());
                        value_str = format!("data: {}\n\n", serde_json::to_string(&value).unwrap());
                        was_correct_output_even_if_error |= json.get("generated_text").is_some();
                    } else if let Some(choices) = json.get("choices") { // openai style
                        let choice0 = &choices[0];
                        let text = choice0.get("text").unwrap().as_str().unwrap().to_string();
                        let stopped = choice0.get("finish_reason").unwrap_or(&json!("")).as_str().unwrap().to_string().starts_with("stop");
                        let mut value: serde_json::Value;
                        (value, finished) = scratch.response_streaming(text, stopped).unwrap();
                        value["created"] = json!(t1.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64 / 1000.0);
                        model_name = json["model"].as_str().unwrap().to_string();
                        value["model"] = json!(model_name.clone());
                        value_str = format!("data: {}\n\n", serde_json::to_string(&value).unwrap());
                    } else {
                        value_str = serde_json::to_string(&json!({"detail": format!("unrecognized response: {:?}", json)})).unwrap();
                    }
                    info!("yield: {:?}", value_str);
                    yield Result::<_, String>::Ok(value_str);
                    if finished {
                        break;
                    }
                },
                Err(err) => {
                    if was_correct_output_even_if_error {
                        // "restream error: Stream ended"
                        break;
                    }
                    info!("restream error: {}\n{:?}", err, err);
                    problem_str = format!("restream error: {}", err);
                    event_source.close();
                    break;
                },
            }
        }
        if !problem_str.is_empty() {
            yield Result::<_, String>::Ok(serde_json::to_string(&json!({"detail": problem_str})).unwrap());
            return;
        } else if !finished && problem_str.is_empty() {
            let mut value: serde_json::Value;
            (value, _) = scratch.response_streaming("".to_string(), false).unwrap();
            value["created"] = json!(t1.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64 / 1000.0);
            value["model"] = json!(model_name.clone());
            let value_str = format!("data: {}\n\n", serde_json::to_string(&value).unwrap());
            info!("yield final: {:?}", value_str);
            yield Result::<_, String>::Ok(value_str);
        }
        info!("yield: [DONE]");
        yield Result::<_, String>::Ok("data: [DONE]\n\n".to_string());
    };

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::wrap_stream(evstream))
        .unwrap();
    return Ok(response);
}
