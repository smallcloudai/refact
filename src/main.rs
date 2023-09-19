use tracing::{error, info};
use std::io::Write;

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
mod telemetry_basic;


#[tokio::main]
async fn main() {
    let _builder1 = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_line_number(true)
        .compact()
        .init();
    let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
    let gcx_maybe = global_context::create_global_context(home_dir).await;
    if let Err(e) = gcx_maybe {
        write!(std::io::stdout(), "URL_NOT_WORKING {}\n", e).unwrap();
        std::io::stdout().flush().unwrap();
        return;
    };
    let gcx = gcx_maybe.unwrap();
    let gcx2 = gcx.clone();
    let caps_reload_task = tokio::spawn(global_context::caps_background_reload(gcx.clone()));
    let tele_backgr_task = tokio::spawn(telemetry_basic::telemetry_background_task(gcx.clone()));
    let server_task = tokio::spawn(async move {
        let gcx_clone = gcx.clone();
        let server = http_server::start_server(gcx_clone);
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
    let _ = server_task.await;  // typically is Err cancelled
    caps_reload_task.abort();
    let _ = caps_reload_task.await;
    tele_backgr_task.abort();
    let _ = tele_backgr_task.await;
    info!("saving telemetry without sending, so should be quick");
    telemetry_basic::telemetry_full_compression_cycle(gcx2.clone(), true).await;
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
