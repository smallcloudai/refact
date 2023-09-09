use tracing::{error, info};
mod cached_tokenizers;
mod scratchpads;
mod forward_to_hf_endpoint;
mod global_context;
mod http_server;


#[tokio::main]
async fn main() {
    let _builder1 = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let global_context = global_context::create_global_context(home_dir);
    let server_task = tokio::spawn(async move {
        let server = http_server::start_server(global_context);
        let server_result = server.await;
        if let Err(e) = server_result {
            error!("server error: {}", e);
        } else {
            info!("clean shutdown");
        }
    });

    tokio::signal::ctrl_c().await.unwrap();
    info!("Ctrl+C");
    server_task.abort();
    let _typically_err_cancelled = server_task.await;
    info!("bb");

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
