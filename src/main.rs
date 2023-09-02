// use reqwest::header::AUTHORIZATION;
// use ropey::Rope;
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;
use std::collections::HashMap;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokenizers::Tokenizer;

use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use hyper::Error;

use tower::ServiceBuilder;

use tracing::{error, info};


async fn hello_world() {
    info!("test test test");
}


#[derive(Debug, Deserialize)]
struct MyRequest {
    model: String,
}


// https://blog.logrocket.com/a-minimal-web-service-in-rust-using-hyper/

// #[derive(Debug)]
// pub struct Context {
//     pub state: AppState,
//     pub req: Request<Body>,
//     pub params: Params,
//     body_bytes: Option<Bytes>,
// }

// route-recognizer = "0.2"
// bytes = "0.5"
// async-trait = "0.1"

// pub async fn param_handler(ctx: Context) -> String {
//     let param = match ctx.params.find("some_param") {
//         Some(v) => v,
//         None => "empty",
//     };
//     format!("param called, param was: {}", param)
// }

// async fn route(
//     router: Arc<Router>,
//     req: Request<hyper::Body>,
//     app_state: AppState,
// ) -> Result<Response, Error> {
//     let found_handler = router.route(req.uri().path(), req.method());
//     let resp = found_handler
//         .handler
//         .invoke(Context::new(app_state, req, found_handler.params))
//         .await;
//     Ok(resp)
// }

// pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
//     let body_bytes = match body_bytes {
//         Some(ref v) => v,
//         _ => {
//             let body = to_bytes(self.req.body_mut()).await?;
//             body_bytes = Some(body);
//             body_bytes.as_ref().expect("body_bytes was set above")
//         }
//     };
//     Ok(serde_json::from_slice(&body_bytes)?)
// }

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Deserialize the request body into the struct
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    let my_request_result = serde_json::from_slice(&body_bytes);
    let my_request: MyRequest = match my_request_result {
        Ok(my_request) => my_request,
        Err(e) => {
            error!("Error deserializing request body: {}", e);
            return Ok(Response::builder()
               .status(hyper::StatusCode::BAD_REQUEST)
              .body(format!("could not parse JSON: {}", e).into())
              .unwrap()
              .into());
        }
    };

    let txt = format!("model was: {}", my_request.model);

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(txt))
        .unwrap();

    Ok(response)
}


#[tokio::main]
async fn main() {
    // let mut stdout = tokio::io::stdout();
    let _builder1 = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    // stdout.write_all(b"Hello, world\n").await.unwrap();
    hello_world().await;

    // let socket = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    let make_svc = make_service_fn(|_conn| {
        async {
            Ok::<_, hyper::Error>(service_fn(handle_request))
        }
    });

    let addr = ([127, 0, 0, 1], 8000).into();
    let server = Server::bind(&addr)
        .serve(make_svc);
    // let server = Server::new(socket, make_svc).serve(addr);

    println!("Server listening on http://{}", addr);


    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    // let http_client = reqwest::Client::new();
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
