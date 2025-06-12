use std::{io::Write, time::Duration};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use axum::{Extension, http::{StatusCode, Uri}, response::IntoResponse};
use hyper::Server;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::{error, info};
use reqwest::{Client, Response};
use serde::Serialize;

use crate::global_context::GlobalContext;
use crate::http::routers::make_refact_http_server;

pub mod routers;

async fn handler_404(path: Uri) -> impl IntoResponse {
    info!("404 {}", path);
    (StatusCode::NOT_FOUND, format!("no handler for {}", path))
}


pub async fn start_server(
    gcx: Arc<ARwLock<GlobalContext>>,
    ask_shutdown_receiver: std::sync::mpsc::Receiver<String>,
) -> Option<JoinHandle<()>> {
    let (port, is_inside_container) = {
        let gcx_locked= gcx.read().await;
        (gcx_locked.cmdline.http_port, gcx_locked.cmdline.inside_container)
    };
    if port == 0 {
        return None
    }
    let shutdown_flag: Arc<AtomicBool> = gcx.read().await.shutdown_flag.clone();
    Some(tokio::spawn(async move {
        let addr = if is_inside_container { ([0, 0, 0, 0], port).into() } else { ([127, 0, 0, 1], port).into() };
        let builder = Server::try_bind(&addr).map_err(|e| {
            let _ = write!(std::io::stderr(), "PORT_BUSY {}\n", e);
            format!("port busy, address {}: {}", addr, e)
        });
        match builder {
            Ok(builder) => {
                info!("HTTP server listening on {}", addr);
                let router = make_refact_http_server().layer(Extension(gcx.clone()));
                let server = builder
                    .serve(router.into_make_service())
                    .with_graceful_shutdown(crate::global_context::block_until_signal(ask_shutdown_receiver, shutdown_flag));
                let resp = server.await.map_err(|e| format!("HTTP server error: {}", e));
                if let Err(e) = resp {
                    error!("server error: {}", e);
                } else {
                    info!("clean shutdown");
                }
            }
            Err(e) => {
                error!("server error: {}", e);
            }
        }
    }))
}

async fn _make_http_request<T: Serialize>(
    method: &str,
    url: &str,
    body: &T,
    max_attempts: usize,
) -> Result<Response, String> {
    // NOTE: if you're going to use https make sure that you set insecure flag from cmdline
    let client = Client::builder().build().map_err(|e| e.to_string())?;
    
    let mut attempt = 1;
    let mut backoff = Duration::from_millis(125);
    loop {

        let request_builder = match method {
            "POST" => client.post(url).json(body),
            "GET" => client.get(url),
            _ => return Err(format!("HTTP method {method} not supported")),
        };
        match request_builder.send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(format!("HTTP request failed with status {}: {}", status, error_text));
                }
                return Ok(response);
            },
            Err(err) => {
                if attempt < max_attempts {
                    tracing::warn!("HTTP request failed, retrying in {}s:\n{}", backoff.as_secs_f64(), err);
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                    backoff *= 2;
                    continue;
                } else {
                    return Err(format!("HTTP request failed after {} attempts: {}", max_attempts, err));
                }
            }
        }
    }
}
