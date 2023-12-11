use std::io::Write;
use std::sync::Arc;

use axum::{Extension, http::{StatusCode, Uri}, response::IntoResponse};
use hyper::Server;
use tokio::signal;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context::GlobalContext;
use crate::http::routers::make_refact_http_server;

pub mod routers;
mod utils;

async fn handler_404(path: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("no handler for {}", path))
}


pub async fn shutdown_signal(ask_shutdown_receiver: std::sync::mpsc::Receiver<String>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
        let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("SIGINT signal received");
        },
        _ = terminate => {},
        _ = tokio::task::spawn_blocking(move || ask_shutdown_receiver.recv()) => {
            info!("graceful shutdown to store telemetry");
        }
    }
}

pub async fn start_server(
    global_context: Arc<ARwLock<GlobalContext>>,
    ask_shutdown_receiver: std::sync::mpsc::Receiver<String>,
) -> Result<(), String> {
    let port = global_context.read().await.cmdline.http_port;
    let addr = ([127, 0, 0, 1], port).into();
    let builder = Server::try_bind(&addr).map_err(|e| {
        write!(std::io::stderr(), "PORT_BUSY {}\n", e).unwrap();
        std::io::stderr().flush().unwrap();
        format!("port busy, address {}: {}", addr, e)
    })?;
    info!("HTTP server listening on {}", addr);
    let router = make_refact_http_server().layer(Extension(global_context.clone()));
    let server = builder
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal(ask_shutdown_receiver));
    let resp = server.await.map_err(|e| format!("HTTP server error: {}", e));
    resp
}
