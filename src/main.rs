use std::io::Write;

use tracing::{error, info, Level};
use tracing_appender;

use crate::background_tasks::start_background_tasks;
use crate::lsp::spawn_lsp_task;
use crate::telemetry::{basic_transmit, snippets_transmit};

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
mod receive_workspace_changes;

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
        // .with_max_level(Level::DEBUG)
        .with_writer(logs_writer)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .with_ansi(false)
        .init();

    {
        info!("cache dir: {}", cache_dir.display());
        info!("started with enduser_client_version==\"{}\"", gcx.read().await.cmdline.enduser_client_version);
        let build_info: std::collections::HashMap<&str, &str> = crate::http::routers::info::get_build_info();
        for (k, v) in build_info {
            info!("{:>20} {}", k, v);
        }
    }

    let mut background_tasks = start_background_tasks(gcx.clone());
    let lsp_task = spawn_lsp_task(gcx.clone(), cmdline.clone()).await;  // execution stays inside if stdin-stdout
    if lsp_task.is_some() {
        background_tasks.push_back(lsp_task.unwrap())
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
    info!("saving telemetry without sending, so should be quick");
    basic_transmit::basic_telemetry_compress(gcx.clone()).await;
    info!("bb\n");
}
