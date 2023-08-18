use tracing::{error, info};
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
mod vecdb_search;
mod lsp;


#[tokio::main]
async fn main() {
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let cache_dir = home_dir.join(".cache").join("refact");
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
    } else {
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::select!{
            _ = ctrl_c => {
                info!("SIGINT signal received");
            }
            _ = tokio::task::spawn_blocking(move || ask_shutdown_receiver.recv()) => {
                info!("graceful shutdown to store telemetry");
            }
        }
    }

    http_server_task.abort();
    let _ = http_server_task.await;  // typically is Err cancelled
    caps_reload_task.abort();
    let _ = caps_reload_task.await;
    tele_backgr_task.abort();
    let _ = tele_backgr_task.await;
    lsp_task.abort();
    let _ = lsp_task.await;
    info!("saving telemetry without sending, so should be quick");
    telemetry_storage::telemetry_full_cycle(gcx4.clone(), true).await;
    info!("bb\n");
}
