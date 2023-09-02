use reqwest::header::AUTHORIZATION;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokenizers::Tokenizer;

use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use tracing::{error, info};
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;



#[tokio::main]
async fn main() {
    let mut stdout = tokio::io::stdout();

    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let cache_dir = home_dir.join(".cache/llm_ls");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .expect("failed to create cache dir");

    let log_file = rolling::never(&cache_dir, "llm-ls.log");
    let builder = tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_line_number(true)
        // .with_env_filter(
        //     EnvFilter::try_from_env("LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info")),
        // )
        ;

    builder
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_span_list(true)
        .init();


    stdout.write_all(b"Hello, world\n").await.unwrap();

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
