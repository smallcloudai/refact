// use ropey::Rope;
use std::convert::Infallible;
use std::net::SocketAddr;
// use serde_json::Error as SerdeJsonError;
use std::collections::HashMap;
use std::path::PathBuf;

use std::sync::Arc;

// use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
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
use crate::scratchpads::call_validation::CodeCompletionPost;


struct GlobalContext {
    http_client: reqwest::Client,
    cache_dir: PathBuf,
    tokenizer_map: Arc<RwLock<HashMap<String, Tokenizer>>>,
}


async fn get_tokenizer(
    global_context: Arc<RwLock<GlobalContext>>,
    model: &str
) -> Result<Tokenizer, ()> {
    let cx_locked = global_context.write().await;
    let mut tokenizer_map_locked = cx_locked.tokenizer_map.write().await;

    let api_key ="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".to_string();
    let tokenizer = cached_tokenizers::get_tokenizer(
        model,
        &mut tokenizer_map_locked,
        &cx_locked.http_client,
        &cx_locked.cache_dir,
        Some(&api_key),
    ).await.unwrap();
    Ok(tokenizer)
}


async fn handle_v1_code_completion(
    global_context: Arc<RwLock<GlobalContext>>,
    body_bytes: hyper::body::Bytes
) -> Result<Response<Body>, hyper::Error> {
    let is_it_valid = serde_json::from_slice::<CodeCompletionPost>(&body_bytes);
    let code_completion_post = match is_it_valid {
        Ok(x) => x,
        Err(e) => {
            error!("Error deserializing request body: {}\n{:?}", e, body_bytes);
            return Ok(Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap()
                .into());
        }
    };

    let t0 = std::time::Instant::now();
    let tokenizer = get_tokenizer(global_context, &code_completion_post.model).await.unwrap();
    info!("get_tokenizer {:?}", t0.elapsed());

    let scratchpad = scratchpads::create_code_completion_scratchpad(
        &tokenizer,
        &code_completion_post,
    );

    let t1 = std::time::Instant::now();
    let prompt_maybe = scratchpad.prompt(
        2048,
        );
    let prompt = match prompt_maybe {
        Ok(x) => x,
        Err(e) => {
            error!("Cannot produce prompt: {}", e);
            return Ok(Response::builder()
               .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
              .body(format!("Cannot produce prompt").into())
              .unwrap()
              .into());
        }
    };
    info!("prompt {:?}\n{}", t1.elapsed(), prompt);

    let txt = "{}";
    info!("handle_v1_code_completion returning: {}", txt);
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(txt))
        .unwrap();
    Ok(response)
}


async fn handle_request(
    global_context: Arc<RwLock<GlobalContext>>,
    remote_addr: SocketAddr,
    path: String,
    method: Method,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    info!("{} {} {} body_bytes={}", remote_addr, method, path, body_bytes.len());
    if method == Method::POST && path == "/v1/code-completion" {
        return handle_v1_code_completion(global_context, body_bytes).await;
    }
    let txt = format!("404 not found, path {}\n", path);
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Body::from(txt))
        .unwrap();
    Ok(response)
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
        tokenizer_map: Arc::new(RwLock::new(HashMap::new())),
    }));

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
