use tracing::{error, info};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::io::Write;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use serde_json::json;
use tokenizers::Tokenizer;

use crate::cached_tokenizers;
use crate::caps;
use crate::scratchpads;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::forward_to_hf_endpoint;
use crate::forward_to_openai_endpoint;

use reqwest_eventsource::Event;
use futures::StreamExt;
use async_stream::stream;

use crate::call_validation::{CodeCompletionPost, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::caps::CodeAssistantCaps;


// https://blog.logrocket.com/a-minimal-web-service-in-rust-using-hyper/
// use route_recognizer::{Match, Params, Router};


fn explain_whats_wrong(status_code: StatusCode, msg: String) -> Response<Body> {
    let body = json!({"detail": msg}).to_string();
    error!("client will see {}", body);
    let response = Response::builder()
       .status(status_code)
       .header("Content-Type", "application/json")
       .body(Body::from(body))
       .unwrap();
    response
}

async fn lookup_code_completion_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    code_completion_post: &CodeCompletionPost,
) -> Result<(String, String, serde_json::Value), String> {
    let cx = global_context.read().await;
    let rec = cx.caps.read().unwrap();
    let (model_name, recommended_model_record) =
        caps::which_model_to_use(
            &rec.code_completion_models,
            &code_completion_post.model,
            &rec.code_completion_default_model,
        )?;
    let (sname, patch) = caps::which_scratchpad_to_use(
        &recommended_model_record.supports_scratchpads,
        &code_completion_post.scratchpad,
        &recommended_model_record.default_scratchpad,
    )?;
    Ok((model_name, sname.clone(), patch.clone()))
}

async fn _get_caps_and_tokenizer(
    global_context: Arc<ARwLock<GlobalContext>>,
    bearer: Option<String>,
    model_name: String,
) -> Result<(Arc<StdRwLock<CodeAssistantCaps>>, Arc<StdRwLock<Tokenizer>>, reqwest::Client), String> {
    let tokenizer_arc: Arc<StdRwLock<Tokenizer>>;
    let caps: Arc<StdRwLock<CodeAssistantCaps>>;
    let client1: reqwest::Client;
    let mut cx_locked = global_context.write().await;
    client1 = cx_locked.http_client.clone();
    let client2 = cx_locked.http_client.clone();
    caps = cx_locked.caps.clone();
    let cache_dir = cx_locked.cache_dir.clone();
    tokenizer_arc = cached_tokenizers::get_tokenizer(
        &mut cx_locked.tokenizer_map,
        &model_name,
        client2,
        &cache_dir,
        bearer.clone(),
    ).await?;
    Ok((caps, tokenizer_arc, client1))
}

async fn handle_v1_code_completion(
    global_context: Arc<ARwLock<GlobalContext>>,
    bearer: Option<String>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, Response<Body>> {
    let mut code_completion_post = serde_json::from_slice::<CodeCompletionPost>(&body_bytes).map_err(|e|
        explain_whats_wrong(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    let (model_name, scratchpad_name, scratchpad_patch) = lookup_code_completion_scratchpad(
        global_context.clone(),
        &code_completion_post,
    ).await.map_err(|e| {
        explain_whats_wrong(StatusCode::BAD_REQUEST, format!("{}", e))
    })?;
    if code_completion_post.parameters.max_new_tokens == 0 {
        code_completion_post.parameters.max_new_tokens = 50;
    }
    let (caps, tokenizer_arc, client1) = _get_caps_and_tokenizer(
        global_context.clone(),
        bearer.clone(),
        model_name.clone(),
    ).await.map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR,format!("Tokenizer: {}", e))
    )?;

    let scratchpad = scratchpads::create_code_completion_scratchpad(
        code_completion_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        tokenizer_arc.clone(),
    ).map_err(|e|
        explain_whats_wrong(StatusCode::BAD_REQUEST, e)
    )?;
    let t1 = std::time::Instant::now();
    let prompt = scratchpad.prompt(
        2048,
        &mut code_completion_post.parameters,
    ).map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;
    // info!("prompt {:?}\n{}", t1.elapsed(), prompt);
    info!("prompt {:?}", t1.elapsed());
    if !code_completion_post.stream {
        _scratchpad_interaction_not_stream(caps, scratchpad, &prompt, model_name, client1, bearer, &code_completion_post.parameters).await
    } else {
        _scratchpad_interaction_stream(caps, scratchpad, &prompt, model_name, client1, bearer, &code_completion_post.parameters).await
    }
}

async fn _scratchpad_interaction_not_stream(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    scratchpad: Box<dyn ScratchpadAbstract>,
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

async fn _scratchpad_interaction_stream(
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


async fn handle_v1_caps(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> Response<Body> {
    let cx = global_context.read().await;
    let caps = cx.caps.read().unwrap();
    let body = json!(*caps).to_string();
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    response
}

async fn handle_request(
    global_context: Arc<ARwLock<GlobalContext>>,
    remote_addr: SocketAddr,
    bearer: Option<String>,
    path: String,
    method: Method,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let t0 = std::time::Instant::now();
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    let mut bearer4log = "none".to_string();
    if let Some(x) = bearer.clone() {
        bearer4log = x.chars().skip(7).take(7).collect::<String>() + "…";
    }
    info!("{} {} {} body_bytes={} bearer={}", remote_addr, method, path, body_bytes.len(), bearer4log);
    let result: Result<Response<Body>, Response<Body>>;
    if method == Method::POST && path == "/v1/code-completion" {
        result = handle_v1_code_completion(global_context, bearer, body_bytes).await;
    } else if method == Method::GET && path == "/v1/caps" {
        result = Ok(handle_v1_caps(global_context).await);
    } else {
        result = Ok(explain_whats_wrong(StatusCode::NOT_FOUND, format!("no handler for {}", path)));
    }
    if let Err(e) = result {
        return Ok(e);
    }
    info!("{} completed in {:?}", path, t0.elapsed());
    return Ok(result.unwrap());
}

pub async fn start_server(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> Result<(), String> {
    let make_svc = make_service_fn(|conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let context_ptr = global_context.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let path = req.uri().path().to_string();
                let method = req.method().clone();
                let context_ptr2 = context_ptr.clone();
                let bearer = req.headers()
                    .get("Authorization")
                    .and_then(|x| x.to_str()
                    .ok()
                    .map(|s| s.to_owned()));
                handle_request(context_ptr2, remote_addr, bearer, path, method, req)
            }))
        }
    });
    let port = global_context.read().await.cmdline.port;
    let addr = ([127, 0, 0, 1], port).into();
    let builder = Server::try_bind(&addr).map_err(|e| {
        write!(std::io::stdout(), "PORT_BUSY {}\n", e).unwrap();
        std::io::stdout().flush().unwrap();
        format!("port busy, address {}: {}", addr, e)
    })?;
    write!(std::io::stdout(), "STARTED port={}\n", port).unwrap();
    std::io::stdout().flush().unwrap();
    let server = builder.serve(make_svc);
    let resp = server.await.map_err(|e| format!("HTTP server error: {}", e));
    resp
}
