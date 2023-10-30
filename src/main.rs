use std::io::Write;

use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_appender;
use crate::background_tasks::start_background_tasks;

mod global_context;
mod caps;
mod call_validation;
mod scratchpads;
mod scratchpad_abstract;
mod forward_to_hf_endpoint;
mod forward_to_openai_endpoint;
mod cached_tokenizers;
mod restream;
mod custom_error;
mod completion_cache;
mod telemetry;
mod vecdb_search;
mod lsp;
mod http;
mod background_tasks;

use crate::telemetry::{basic_transmit, snippets_transmit};


#[tokio::main]
async fn main() {
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let cache_dir = home_dir.join(".cache/refact");
    let (gcx, ask_shutdown_receiver, cmdline) = global_context::create_global_context(cache_dir.clone()).await;
    let (logs_writer, _guard) = if cmdline.logs_stderr {
        tracing_appender::non_blocking(std::io::stderr())
    } else {
        write!(std::io::stderr(), "This rust binary keeps logs as files, rotated daily. Try\ntail -f {}/logs/\nor use --logs-stderr for debugging.\n\n", cache_dir.display()).unwrap();
        tracing_appender::non_blocking(tracing_appender::rolling::RollingFileAppender::new(
            tracing_appender::rolling::Rotation::DAILY,
            cache_dir.join("logs"),
            "rustbinary",
        ))
    };
    let _tracing = tracing_subscriber::fmt()
        .with_writer(logs_writer)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    info!("started");
    info!("cache dir: {}", cache_dir.display());

    let gcx2 = gcx.clone();
    let gcx3 = gcx.clone();
    let gcx4 = gcx.clone();
    let background_tasks = start_background_tasks(gcx.clone());

    let lsp_task = tokio::spawn(async move {
        if cmdline.lsp_stdin_stdout == 0 && cmdline.lsp_port > 0 {
            let addr: std::net::SocketAddr = ([127, 0, 0, 1], cmdline.lsp_port).into();
            let listener: TcpListener = TcpListener::bind(&addr).await.unwrap();
            info!("LSP listening on {}", listener.local_addr().unwrap());
            loop {
                // possibly wrong code, look at
                // tower-lsp-0.20.0/examples/tcp.rs
                match listener.accept().await {
                    Ok((s, addr)) => {
                        info!("LSP new client connection from {}", addr);
                        let (read, write) = tokio::io::split(s);
                        let (lsp_service, socket) = lsp::build_lsp_service(gcx2.clone());
                        tower_lsp::Server::new(read, write, socket).serve(lsp_service).await;
                    }
                    Err(e) => {
                        error!("Error accepting client connection: {}", e);
                    }
                }
            }
        }
    });

    if cmdline.lsp_stdin_stdout != 0 && cmdline.lsp_port == 0 {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (lsp_service, socket) = lsp::build_lsp_service(gcx3.clone());
        tower_lsp::Server::new(stdin, stdout, socket).serve(lsp_service).await;
        info!("LSP loop exit");
    }

    let gcx_clone = gcx.clone();
    let server = http::start_server(gcx_clone, ask_shutdown_receiver);
    let server_result = server.await;
    if let Err(e) = server_result {
        error!("server error: {}", e);
    } else {
        info!("clean shutdown");
    }

    background_tasks.abort().await;
    lsp_task.abort();
    let _ = lsp_task.await;
    info!("saving telemetry without sending, so should be quick");
    basic_transmit::telemetry_full_cycle(gcx4.clone(), true).await;
    info!("bb\n");
}
