use tracing::{error, info};
use std::convert::Infallible;
use std::net::SocketAddr;
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
use crate::recommendations;
use crate::scratchpads;
use crate::forward_to_hf_endpoint;
use crate::call_validation::CodeCompletionPost;
use crate::global_context::GlobalContext;


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
    let rec = cx.recommendations.read().unwrap();
    let (model_name, recommended_model_record) =
        recommendations::which_model_to_use(
            &rec.code_completion_models,
            &code_completion_post.model,
            &rec.code_completion_default_model,
        )?;
    let (sname, patch) = recommendations::which_scratchpad_to_use(
        &recommended_model_record.supports_scratchpads,
        &code_completion_post.scratchpad,
        &recommended_model_record.default_scratchpad,
    )?;
    Ok((model_name, sname.clone(), patch.clone()))
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
    let tokenizer_arc: Arc<StdRwLock<Tokenizer>>;
    let client1: reqwest::Client;
    let client2: reqwest::Client;
    {
        let mut cx_locked = global_context.write().await;
        client1 = cx_locked.http_client.clone();
        client2 = cx_locked.http_client.clone();
        let cache_dir = cx_locked.cache_dir.clone();
        tokenizer_arc = cached_tokenizers::get_tokenizer(
            &mut cx_locked.tokenizer_map,
            &model_name,
            client2,
            &cache_dir,
            bearer.clone(),
        ).await.map_err(|e|
            explain_whats_wrong(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Tokenizer: {}", e))
        )?;
    }

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

    let t2 = std::time::Instant::now();
    let hf_endpoint_result = forward_to_hf_endpoint::simple_forward_to_hf_endpoint_no_streaming(
        bearer.clone(),
        &model_name,
        &prompt,
        &client1,
        &code_completion_post.parameters,
    ).await.map_err(|e|
        explain_whats_wrong(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("forward_to_hf_endpoint: {}", e))
    )?;
    info!("forward_to_hf_endpoint {:?}", t2.elapsed());
    let tuple_json_finished = scratchpad.re_stream_response(hf_endpoint_result)
        .map_err(|e|
            explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("re_stream_response: {}", e))
    )?;
    let txt = serde_json::to_string(&tuple_json_finished.0).unwrap();
    info!("handle_v1_code_completion return {}", txt);
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(txt))
        .unwrap();
    Ok(response)
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
    let addr = ([127, 0, 0, 1], 8001).into();
    let server = Server::bind(&addr).serve(make_svc);
    info!("Server listening on http://{}", addr);
    let resp = server.await.map_err(|e| format!("HTTP server error: {}", e));
    resp
}
