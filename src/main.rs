use tracing::{error, info};
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
mod telemetry_basic;
use std::io::Write;


#[tokio::main]
async fn main() {
    let _builder1 = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let global_context_maybe = global_context::create_global_context(home_dir).await;
    if let Err(e) = global_context_maybe {
        write!(std::io::stdout(), "URL_NOT_WORKING {}\n", e).unwrap();
        std::io::stdout().flush().unwrap();
        return;
    };
    let global_context = global_context_maybe.unwrap();
    tokio::spawn(global_context::caps_background_reload(global_context.clone()));
    tokio::spawn(telemetry_basic::telemetry_background_task(global_context.clone()));

    let server = http_server::start_server(global_context);
    let server_result = server.await;
    if let Err(e) = server_result {
        error!("server error: {}", e);
    } else {
        info!("clean shutdown");
    }

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
