// use ropey::Rope;
use std::convert::Infallible;
use std::net::SocketAddr;
// use serde_json::Error as SerdeJsonError;
use std::collections::HashMap;
use std::path::PathBuf;

use std::sync::Arc;


// use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use std::sync::RwLock as StdRwLock;

// use async_trait::async_trait;
use tokenizers::Tokenizer;

use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};

use tracing::{error, info};

// https://blog.logrocket.com/a-minimal-web-service-in-rust-using-hyper/
// use route_recognizer::{Match, Params, Router};

mod cached_tokenizers;
mod scratchpads;
mod forward_to_hf_endpoint;
use crate::scratchpads::call_validation::CodeCompletionPost;
use serde_json::json;


struct GlobalContext {
    http_client: reqwest::Client,
    cache_dir: PathBuf,
    tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
}


fn explain_whats_wrong(status_code: StatusCode, msg: String) -> Response<Body> {
    error!("{:?}", msg);
    let body = json!({"detail": msg}).to_string();
    let response = Response::builder()
       .status(status_code)
       .header("Content-Type", "application/json")
       .body(Body::from(body))
       .unwrap();
    response
}


async fn handle_v1_code_completion(
    global_context: Arc<RwLock<GlobalContext>>,
    bearer: Option<String>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, Response<Body>> {
    let mut code_completion_post = serde_json::from_slice::<CodeCompletionPost>(&body_bytes).map_err(|e|
        explain_whats_wrong(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
    )?;
    if code_completion_post.model.is_empty() {
        code_completion_post.model = "bigcode/starcoder".to_string();
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
            &code_completion_post.model,
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
            tokenizer_arc.clone(),
            code_completion_post.clone(),
        );
    let t1 = std::time::Instant::now();
    let prompt = scratchpad.prompt(2048).map_err(|e|
        explain_whats_wrong(StatusCode::INTERNAL_SERVER_ERROR, format!("Prompt: {}", e))
    )?;
    // info!("prompt {:?}\n{}", t1.elapsed(), prompt);
    info!("prompt {:?}", t1.elapsed());

    let t2 = std::time::Instant::now();
    let hf_endpoint_result = forward_to_hf_endpoint::simple_forward_to_hf_endpoint_no_streaming(
        &code_completion_post.model,
        &prompt,
        &client1,
        bearer.clone(),
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
    global_context: Arc<RwLock<GlobalContext>>,
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


#[tokio::main]
async fn main() {
    let _builder1 = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let global_context = Arc::new(RwLock::new(GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir: home_dir.join(".cache/refact"),
        tokenizer_map: HashMap::new(),
    }));

    let make_svc = make_service_fn(|conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let context_ptr = global_context.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let path = req.uri().path().to_string();
                let method = req.method().clone();
                let context_ptr2 = context_ptr.clone();
                let bearer = req.headers().get("Authorization").and_then(|x| x.to_str().ok().map(|s| s.to_owned()));
                handle_request(context_ptr2, remote_addr, bearer, path, method, req)
            }))
        }
    });
    let addr = ([127, 0, 0, 1], 8001).into();
    let server = Server::bind(&addr).serve(make_svc);
    println!("Server listening on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    // let (service, socket) = LspService::build(|client| Backend {
    //     cache_dir,
    //     client,
    //     document_map: Arc::new(RwLock::new(HashMap::new())),
    //     http_client,
    //     workspace_folders: Arc::new(RwLock::new(None)),
    //     tokenizer_map: Arc::new(RwLock::new(HashMap::new())),
    // })
    // .custom_method("llm-ls/getCompletions", Backend::get_completions)
    // .finish();
    // Server::new(stdin, stdout, socket).serve(service).await;
}
