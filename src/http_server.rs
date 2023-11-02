use tracing::info;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::io::Write;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use serde_json::json;

use crate::caps;
use crate::scratchpads;
use crate::call_validation::{CodeCompletionPost, ChatPost};
use crate::global_context::GlobalContext;
use crate::caps::CodeAssistantCaps;
use crate::custom_error::ScratchError;
use crate::telemetry::telemetry_structs;
use crate::telemetry::snippets_collection;
use crate::completion_cache;


async fn _lookup_code_completion_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    code_completion_post: &CodeCompletionPost,
) -> Result<(String, String, serde_json::Value), String> {
    let caps_locked = caps.read().unwrap();
    let (model_name, recommended_model_record) =
        caps::which_model_to_use(
            &caps_locked.code_completion_models,
            &code_completion_post.model,
            &caps_locked.code_completion_default_model,
        )?;
    let (sname, patch) = caps::which_scratchpad_to_use(
        &recommended_model_record.supports_scratchpads,
        &code_completion_post.scratchpad,
        &recommended_model_record.default_scratchpad,
    )?;
    Ok((model_name, sname.clone(), patch.clone()))
}

async fn _lookup_chat_scratchpad(
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    chat_post: &ChatPost,
) -> Result<(String, String, serde_json::Value), String> {
    let caps_locked = caps.read().unwrap();
    let (model_name, recommended_model_record) =
        caps::which_model_to_use(
            &caps_locked.code_chat_models,
            &chat_post.model,
            &caps_locked.code_chat_default_model,
        )?;
    let (sname, patch) = caps::which_scratchpad_to_use(
        &recommended_model_record.supports_scratchpads,
        &chat_post.scratchpad,
        &recommended_model_record.default_scratchpad,
    )?;
    Ok((model_name, sname.clone(), patch.clone()))
}

pub async fn handle_v1_code_completion(
    global_context: Arc<ARwLock<GlobalContext>>,
    code_completion_post: &mut CodeCompletionPost
) -> Result<Response<Body>, ScratchError> {
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone()).await?;
    let (model_name, scratchpad_name, scratchpad_patch) = _lookup_code_completion_scratchpad(
        caps.clone(),
        &code_completion_post,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e))
    })?;
    if code_completion_post.parameters.max_new_tokens == 0 {
        code_completion_post.parameters.max_new_tokens = 50;
    }
    if code_completion_post.model == "" {
        code_completion_post.model = model_name.clone();
    }
    if code_completion_post.scratchpad == "" {
        code_completion_post.scratchpad = scratchpad_name.clone();
    }
    code_completion_post.parameters.temperature = Some(code_completion_post.parameters.temperature.unwrap_or(0.2));
    let (client1, api_key, cache_arc, tele_storage) = {
        let cx_locked = global_context.write().await;
        (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone(), cx_locked.completions_cache.clone(), cx_locked.telemetry.clone())
    };
    if !code_completion_post.no_cache {
        let cache_key = completion_cache::cache_key_from_post(&code_completion_post);
        let cached_maybe = completion_cache::cache_get(cache_arc.clone(), cache_key.clone());
        if let Some(cached_json_value) = cached_maybe {
            // info!("cache hit for key {:?}", cache_key.clone());
            if !code_completion_post.stream {
                return crate::restream::cached_not_stream(&cached_json_value).await;
            } else {
                return crate::restream::cached_stream(&cached_json_value).await;
            }
        }
    }

    let mut scratchpad = scratchpads::create_code_completion_scratchpad(
        global_context.clone(),
        caps,
        model_name.clone(),
        code_completion_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        cache_arc.clone(),
        tele_storage.clone(),
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    let t1 = std::time::Instant::now();
    let prompt = scratchpad.prompt(
        2048,
        &mut code_completion_post.parameters,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;
    // info!("prompt {:?}\n{}", t1.elapsed(), prompt);
    info!("prompt {:?}", t1.elapsed());
    if !code_completion_post.stream {
        crate::restream::scratchpad_interaction_not_stream(global_context.clone(), scratchpad, "completion".to_string(), &prompt, model_name, client1, api_key, &code_completion_post.parameters).await
    } else {
        crate::restream::scratchpad_interaction_stream(global_context.clone(), scratchpad, "completion-stream".to_string(), prompt, model_name, client1, api_key, code_completion_post.parameters.clone()).await
    }
}

pub async fn handle_v1_code_completion_web(
    global_context: Arc<ARwLock<GlobalContext>>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, ScratchError> {
    let mut code_completion_post = serde_json::from_slice::<CodeCompletionPost>(&body_bytes).map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    handle_v1_code_completion(global_context.clone(), &mut code_completion_post).await
}


async fn handle_v1_chat(
    global_context: Arc<ARwLock<GlobalContext>>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, ScratchError> {
    let mut chat_post = serde_json::from_slice::<ChatPost>(&body_bytes).map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone()).await?;
    let (model_name, scratchpad_name, scratchpad_patch) = _lookup_chat_scratchpad(
        caps.clone(),
        &chat_post,
    ).await.map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("{}", e))
    })?;
    if chat_post.parameters.max_new_tokens == 0 {
        chat_post.parameters.max_new_tokens = 2048;
    }
    chat_post.parameters.temperature = Some(chat_post.parameters.temperature.unwrap_or(0.2));
    chat_post.model = model_name.clone();
    let (client1, api_key) = {
        let cx_locked = global_context.write().await;
        (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone())
    };
    let vecdb_search = global_context.read().await.vecdb_search.clone();
    let mut scratchpad = scratchpads::create_chat_scratchpad(
        global_context.clone(),
        caps,
        model_name.clone(),
        chat_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        vecdb_search,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::BAD_REQUEST, e)
    )?;
    let t1 = std::time::Instant::now();
    let prompt = scratchpad.prompt(
        2048,
        &mut chat_post.parameters,
    ).await.map_err(|e|
        ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;
    // info!("chat prompt {:?}\n{}", t1.elapsed(), prompt);
    info!("chat prompt {:?}", t1.elapsed());
    crate::restream::scratchpad_interaction_stream(
        global_context.clone(),
        scratchpad,
        "chat-stream".to_string(),
        prompt,
        model_name,
        client1,
        api_key,
        chat_post.parameters.clone()
    ).await
}


async fn handle_v1_telemetry_network(
    global_context: Arc<ARwLock<GlobalContext>>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<telemetry_structs::TelemetryNetwork>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    global_context.write().await.telemetry.write().unwrap().tele_net.push(post);
    Ok(Response::builder()
       .status(StatusCode::OK)
       .body(Body::from(json!({"success": 1}).to_string()))
       .unwrap())
}

async fn handle_v1_snippet_accepted(
    global_context: Arc<ARwLock<GlobalContext>>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<snippets_collection::SnippetAccepted>(&body_bytes).map_err(|e| {
        ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    })?;
    let success = snippets_collection::snippet_accepted(global_context.clone(), post.snippet_telemetry_id).await;
    Ok(Response::builder()
      .status(StatusCode::OK)
      .body(Body::from(json!({"success": success}).to_string()))
      .unwrap())
}

async fn handle_v1_caps(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> Result<Response<Body>, ScratchError> {
    let caps_result = crate::global_context::try_load_caps_quickly_if_not_present(global_context.clone()).await;
    let caps = match caps_result {
        Ok(x) => x,
        Err(e) => {
            return Err(ScratchError::new(StatusCode::SERVICE_UNAVAILABLE, format!("{}", e)));
        }
    };
    let caps_locked = caps.read().unwrap();
    let body = json!(*caps_locked).to_string();
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();
    Ok(response)
}


async fn handle_request(
    global_context: Arc<ARwLock<GlobalContext>>,
    remote_addr: SocketAddr,
    path: String,
    method: Method,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let t0 = std::time::Instant::now();
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    info!("{} {} {} body_bytes={}", remote_addr, method, path, body_bytes.len());
    let result: Result<Response<Body>, ScratchError>;
    if method == Method::POST && path == "/v1/code-completion" {
        result = handle_v1_code_completion_web(global_context.clone(), body_bytes).await;
    } else if method == Method::POST && path == "/v1/chat" {
        result = handle_v1_chat(global_context.clone(), body_bytes).await;
    } else if method == Method::POST && path == "/v1/telemetry-network" {
        result = handle_v1_telemetry_network(global_context.clone(), body_bytes).await;
    } else if method == Method::POST && path == "/v1/snippet-accepted" {
        result = handle_v1_snippet_accepted(global_context.clone(), body_bytes).await;
    } else if method == Method::GET && path == "/v1/caps" {
        result = handle_v1_caps(global_context.clone()).await;
    } else if method == Method::GET && path == "/v1/graceful-shutdown" {
        let gcx_locked = global_context.read().await;
        gcx_locked.ask_shutdown_sender.lock().unwrap().send(format!("going-down")).unwrap();
        result = Ok(Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(json!({"success": true}).to_string()))
            .unwrap());
    } else {
        result = Err(ScratchError::new(StatusCode::NOT_FOUND, format!("no handler for {}", path)));
    }
    if let Err(e) = result {
        if !e.telemetry_skip {
            let tele_storage = &global_context.read().await.telemetry;
            let mut tele_storage_locked = tele_storage.write().unwrap();
            tele_storage_locked.tele_net.push(telemetry_structs::TelemetryNetwork::new(
                path.clone(),
                format!("{}", method),
                false,
                format!("{}", e.message),
            ));
        }
        return Ok(e.to_response());
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
                handle_request(context_ptr2, remote_addr, path, method, req)
            }))
        }
    });
    let port = global_context.read().await.cmdline.http_port;
    let addr = ([127, 0, 0, 1], port).into();
    let builder = Server::try_bind(&addr).map_err(|e| {
        write!(std::io::stderr(), "PORT_BUSY {}\n", e).unwrap();
        std::io::stderr().flush().unwrap();
        format!("port busy, address {}: {}", addr, e)
    })?;
    info!("HTTP server listening on {}", addr);
    let server = builder.serve(make_svc);
    let resp = server.await.map_err(|e| format!("HTTP server error: {}", e));
    resp
}
