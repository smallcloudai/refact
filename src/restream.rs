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
    let t2 = std::time::Instant::now();
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
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("forward_to_hf_endpoint: {}", e))
    )?;
    info!("forward to endpoint {:?}", t2.elapsed());

    let scratchpad_result: Result<serde_json::Value, String>;
    if let Some(hf_arr) = model_says.as_array() {
        let choices = hf_arr.iter()
            .map(|x| {
                x.get("generated_text").unwrap().as_str().unwrap().to_string()
            }).collect::<Vec<_>>();
        scratchpad_result = scratchpad.response_n_choices(choices);

    } else if let Some(oai_choices) = model_says.get("choices") {
        let choices = oai_choices.as_array().unwrap().iter()
            .map(|x| {
                x.get("text").unwrap().as_str().unwrap().to_string()
            }).collect::<Vec<_>>();
        scratchpad_result = scratchpad.response_n_choices(choices);
        // TODO: "model", "finish_reason"?

    } else if let Some(err) = model_says.get("error") {
        return Ok(explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,
            format!("model says: {:?}", err)
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

    let txt = serde_json::to_string(&scratchpad_result.unwrap()).unwrap();
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
    model_name: String,
    client: reqwest::Client,
    bearer: Option<String>,
    parameters: &SamplingParameters,
) -> Result<Response<Body>, Response<Body>> {
    let t1 = std::time::Instant::now();
    let (endpoint_style, endpoint_template) = {
        let caps_locked = caps.read().unwrap();
        (caps_locked.endpoint_style.clone(), caps_locked.endpoint_template.clone())
    };
    let mut event_source = forward_to_hf_endpoint::forward_to_hf_style_endpoint_streaming(
        bearer.clone(),
        &model_name,
        &prompt,
        &client,
        &endpoint_template,
        &parameters,
    ).await.map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("forward_to_hf_endpoint: {}", e))
    )?;

    let stream3 = stream! {
        let scratch = &mut scratchpad;
        // let my_event_source = &mut event_source;
        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => {},
                Ok(Event::Message(message)) => {
                    println!("Message: {:#?}", message);
                    let json = serde_json::from_str::<serde_json::Value>(&message.data).unwrap();
                    // info!("json: {:?}", json);
                    if let Some(token) = json.get("token") {
                        // info!("got token: {:?}", token);
                        let text = token.get("text").unwrap().as_str().unwrap().to_string();
                        // info!("text: {:?}", text);
                        let (value, finished) = scratch.response_streaming(text).unwrap();
                        let value_str = serde_json::to_string(&value).unwrap();
                        info!("yield: {:?}", value_str);
                        yield Result::<_, String>::Ok(format!("data: {}\n\n", value_str));
                        if finished {
                            break;
                        }
                    } else {
                        info!("unrecognized response: {:?}", json);
                    }
                },
                Err(err) => {
                    println!("Error: {}", err);
                    event_source.close();
                },
            }
        }
        info!("yield: DONE");
        yield Result::<_, String>::Ok("data: [DONE]\n\n".to_string());
    };
    // pin_mut!(stream3); // needed for iteration

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::wrap_stream(stream3))
        .unwrap();
    return Ok(response);
}

