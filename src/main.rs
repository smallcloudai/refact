use tracing::{error, info};
use tower_lsp::{LspService, Server};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tokio::net::TcpListener;
use std::io::Write;
use tracing_appender;

mod global_context;
mod caps;
mod call_validation;
mod scratchpads;
mod scratchpad_abstract;
mod forward_to_hf_endpoint;
mod forward_to_openai_endpoint;
mod cached_tokenizers;
mod http_server;
mod restream;
mod custom_error;
mod completion_cache;
mod telemetry_basic;
mod telemetry_snippets;
mod telemetry_storage;
mod lsp;


#[tokio::main]
async fn main() {
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let cache_dir = home_dir.join(".cache/refact");
    let (gcx, cmdline) = global_context::create_global_context(cache_dir.clone()).await;
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

    let gcx2 = gcx.clone();
    let gcx3 = gcx.clone();
    let caps_reload_task = tokio::spawn(global_context::caps_background_reload(gcx.clone()));
    let tele_backgr_task = tokio::spawn(telemetry_storage::telemetry_background_task(gcx.clone()));
    let http_server_task = tokio::spawn(async move {
        let gcx_clone = gcx.clone();
        let server = http_server::start_server(gcx_clone);
        let server_result = server.await;
        if let Err(e) = server_result {
            error!("server error: {}", e);
        } else {
            info!("clean shutdown");
        }
    });

    let lsp_task = tokio::spawn(async move {
        if cmdline.lsp_port > 0 {
            let addr: std::net::SocketAddr = ([127, 0, 0, 1], cmdline.lsp_port).into();
            let listener: TcpListener = TcpListener::bind(&addr).await.unwrap();
            info!("LSP listening on {}", listener.local_addr().unwrap());
            loop {
                // possibly wrong code, look at
                // tower-lsp-0.20.0/examples/tcp.rs
                match listener.accept().await {
                    Ok((s, addr)) => {
                        info!("new client connection from {}", addr);
                        let (read, write) = tokio::io::split(s);
                        // #[cfg(feature = "runtime-agnostic")]
                        // let (read, write) = (read.compat(), write.compat_write());
                        let (lsp_service, socket) = LspService::new(|client| lsp::Backend {
                            gcx: gcx2.clone(),
                            client,
                            document_map: Arc::new(ARwLock::new(HashMap::new())),
                            workspace_folders: Arc::new(ARwLock::new(None)),
                        });
                        Server::new(read, write, socket).serve(lsp_service).await;
                    }
                    Err(e) => {
                        error!("Error accepting client connection: {}", e);
                    }
                }
            }
        } else if cmdline.lsp_stdin_stdout != 0 {
            let (service, socket) = LspService::build(|client| lsp::Backend {
                gcx: gcx2.clone(),
                client,
                document_map: Arc::new(ARwLock::new(HashMap::new())),
                workspace_folders: Arc::new(ARwLock::new(None)),
            })
            .custom_method("llm-ls/getCompletions", lsp::Backend::get_completions)
            .finish();
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            Server::new(stdin, stdout, socket).serve(service).await;
        }
    });

    tokio::signal::ctrl_c().await.unwrap();  // most of the time is spent here

    info!("Ctrl+C");
    http_server_task.abort();
    let _ = http_server_task.await;  // typically is Err cancelled
    caps_reload_task.abort();
    let _ = caps_reload_task.await;
    tele_backgr_task.abort();
    let _ = tele_backgr_task.await;
    lsp_task.abort();
    let _ = lsp_task.await;
    info!("saving telemetry without sending, so should be quick");
    telemetry_storage::telemetry_full_cycle(gcx3.clone(), true).await;
    info!("bb");
}
