use std::io::Write;

use tokio::task::JoinHandle;
use tracing::{info, Level};
use tracing_appender;
use std::panic;
use backtrace;

use crate::background_tasks::start_background_tasks;
use crate::lsp::spawn_lsp_task;
use crate::telemetry::{basic_transmit, snippets_transmit};

mod version;
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
mod lsp;
mod http;
mod background_tasks;
mod known_models;
mod dashboard;
mod files_in_workspace;
mod files_in_jsonl;
mod files_correction;
mod vecdb;
mod fetch_embedding;
mod at_commands;
mod at_tools;
mod nicer_logs;
mod toolbox;
mod ast;


#[tokio::main]
async fn main() {
    let cpu_num = std::thread::available_parallelism().unwrap().get();
    rayon::ThreadPoolBuilder::new().num_threads(cpu_num / 2).build_global().unwrap();
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let cache_dir = home_dir.join(".cache/refact");
    let (gcx, ask_shutdown_receiver, shutdown_flag, cmdline) = global_context::create_global_context(cache_dir.clone()).await;
    let (logs_writer, _guard) = if cmdline.logs_stderr {
        tracing_appender::non_blocking(std::io::stderr())
    } else {
        let _ = write!(std::io::stderr(), "This rust binary keeps logs as files, rotated daily. Try\ntail -f {}/logs/\nor use --logs-stderr for debugging.\n\n", cache_dir.display());
        tracing_appender::non_blocking(tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("rustbinary")
            .max_log_files(30)
            .build(cache_dir.join("logs")).unwrap()
        )
    };
    let _tracing = tracing_subscriber::fmt()
        .with_max_level(if cmdline.verbose { Level::DEBUG } else { Level::INFO })
        .with_writer(logs_writer)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .with_ansi(false)
        .init();
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = backtrace::Backtrace::new();
        tracing::error!("Panic occurred: {:?}\n{:?}", panic_info, backtrace);
    }));

    {
        info!("cache dir: {}", cache_dir.display());
        info!("started with enduser_client_version==\"{}\"", gcx.read().await.cmdline.enduser_client_version);
        let build_info: std::collections::HashMap<&str, &str> = crate::http::routers::info::get_build_info();
        for (k, v) in build_info {
            info!("{:>20} {}", k, v);
        }
    }
    files_in_workspace::enqueue_all_files_from_workspace_folders(gcx.clone(), true, false).await;
    files_in_jsonl::enqueue_all_docs_from_jsonl_but_read_first(gcx.clone(), true, false).await;

    let mut background_tasks = start_background_tasks(gcx.clone()).await;
    // vector db will spontaneously start if the downloaded caps and command line parameters are right

    let should_start_http = cmdline.http_port != 0;
    let should_start_lsp = (cmdline.lsp_port == 0 && cmdline.lsp_stdin_stdout == 1) ||
        (cmdline.lsp_port != 0 && cmdline.lsp_stdin_stdout == 0);

    // not really needed, but it's nice to have an error message sooner if there's one
    let _caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0).await;

    let mut main_handle: Option<JoinHandle<()>> = None;
    if should_start_http {
        main_handle = http::start_server(gcx.clone(), ask_shutdown_receiver, shutdown_flag).await;
    }
    if should_start_lsp {
        if main_handle.is_none() {
            // FIXME: this ignores crate::global_context::block_until_signal , important because now we have a database to corrupt
            main_handle = spawn_lsp_task(gcx.clone(), cmdline.clone()).await;
        } else {
            background_tasks.push_back(spawn_lsp_task(gcx.clone(), cmdline.clone()).await.unwrap())
        }
    }
    if main_handle.is_some() {
        let _ = main_handle.unwrap().await;
    }

    background_tasks.abort().await;
    info!("saving telemetry without sending, so should be quick");
    basic_transmit::basic_telemetry_compress(gcx.clone()).await;
    info!("bb\n");
}
