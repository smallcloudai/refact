use tracing::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::RwLock as StdRwLock;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;
use structopt::StructOpt;
use std::io::Write;
use crate::caps::CodeAssistantCaps;
use crate::completion_cache::CompletionCache;
use crate::telemetry::telemetry_structs;
use crate::custom_error::ScratchError;
use hyper::StatusCode;
use tower_lsp::lsp_types::WorkspaceFolder;
use crate::receive_workspace_changes::Document;
use crate::vecdb::vecdb::VecDb;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, help="Send logs to stderr, as opposed to ~/.cache/refact/logs, so it's easier to debug.")]
    pub logs_stderr: bool,
    #[structopt(long, short="u", help="URL to start working. The first step is to fetch coding_assistant_caps.json.")]
    pub address_url: String,
    #[structopt(long, short="k", default_value="", help="The API key to authenticate your requests, will appear in HTTP requests this binary makes.")]
    pub api_key: String,
    #[structopt(long, short="p", default_value="8001", help="Bind 127.0.0.1:<port> to listen for HTTP requests, such as /v1/code-completion, /v1/chat, /v1/caps.")]
    pub http_port: u16,
    #[structopt(long, default_value="", help="End-user client version, such as version of VS Code plugin.")]
    pub enduser_client_version: String,
    #[structopt(long, short="b", help="Send basic telemetry (counters and errors)")]
    pub basic_telemetry: bool,
    #[structopt(long, short="s", help="Send snippet telemetry (code snippets)")]
    pub snippet_telemetry: bool,
    #[structopt(long, default_value="0", help="Bind 127.0.0.1:<port> and act as an LSP server. This is compatible with having an HTTP server at the same time.")]
    pub lsp_port: u16,
    #[structopt(long, default_value="0", help="Act as an LSP server, use stdin stdout for communication. This is compatible with having an HTTP server at the same time. But it's not compatible with LSP port.")]
    pub lsp_stdin_stdout: u16,
    #[structopt(long, help="Trust self-signed SSL certificates")]
    pub insecure: bool,
    #[structopt(long, help="Whether to use a vector database")]
    pub vecdb: bool,
    #[structopt(long, short = "f", default_value = "", help = "The path to jsonl file which contains filtered source files")]
    pub files_set_path: String,
    #[structopt(long, default_value = "", help = "Vecdb forced path")]
    pub vecdb_forced_path: String,
}


pub struct Slowdown {
    // Be nice to cloud/self-hosted, don't flood it
    pub requests_in_flight: u64,
}

pub struct LSPBackendDocumentState {
    pub document_map: Arc<ARwLock<HashMap<String, Document>>>,
    pub workspace_folders: Arc<ARwLock<Option<Vec<WorkspaceFolder>>>>,
}

// #[derive(Debug)]
pub struct GlobalContext {
    pub cmdline: CommandLine,
    pub http_client: reqwest::Client,
    pub http_client_slowdown: Arc<Mutex<Slowdown>>,
    pub cache_dir: PathBuf,
    pub caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>,
    pub caps_last_attempted_ts: u64,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub completions_cache: Arc<StdRwLock<CompletionCache>>,
    pub telemetry: Arc<StdRwLock<telemetry_structs::Storage>>,
    pub vec_db: Arc<AMutex<Option<VecDb>>>,
    pub ask_shutdown_sender: Arc<Mutex<std::sync::mpsc::Sender<String>>>,
    pub lsp_backend_document_state: LSPBackendDocumentState,
}

pub type SharedGlobalContext = Arc<ARwLock<GlobalContext>>;  // TODO: remove this type alias, confusing

const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn try_load_caps_quickly_if_not_present(
    global_context: Arc<ARwLock<GlobalContext>>,
    max_age_seconds: u64,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, ScratchError> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let caps_last_attempted_ts;
    let max_age = if max_age_seconds > 0 { max_age_seconds } else { CAPS_BACKGROUND_RELOAD };
    {
        let mut cx_locked = global_context.write().await;
        if cx_locked.caps_last_attempted_ts + max_age < now {
            cx_locked.caps = None;
            cx_locked.caps_last_attempted_ts = 0;
            caps_last_attempted_ts = 0;
        } else {
            if let Some(caps_arc) = cx_locked.caps.clone() {
                return Ok(caps_arc.clone());
            }
            caps_last_attempted_ts = cx_locked.caps_last_attempted_ts;
        }
    }
    if caps_last_attempted_ts + CAPS_RELOAD_BACKOFF > now {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "server is not reachable, no caps available".to_string()));
    }
    let caps_result = crate::caps::load_caps(
        CommandLine::from_args(),
        global_context.clone()
    ).await;
    {
        let mut global_context_locked = global_context.write().await;
        global_context_locked.caps_last_attempted_ts = now;
        match caps_result {
            Ok(caps) => {
                global_context_locked.caps = Some(caps.clone());
                info!("quick load caps successful");
                write!(std::io::stderr(), "CAPS\n").unwrap();
                Ok(caps)
            },
            Err(e) => {
                return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("server is not reachable: {}", e)));
            }
        }
    }
}

pub async fn look_for_piggyback_fields(
    global_context: Arc<ARwLock<GlobalContext>>,
    anything_from_server: &serde_json::Value)
{
    let mut global_context_locked = global_context.write().await;
    if let Some(dict) = anything_from_server.as_object() {
        let new_caps_version = dict.get("caps_version").and_then(|v| v.as_i64()).unwrap_or(0);
        if new_caps_version > 0 {
            if let Some(caps) = global_context_locked.caps.clone() {
                let caps_locked = caps.read().unwrap();
                if caps_locked.caps_version < new_caps_version {
                    info!("detected biggyback caps version {} is newer than the current version {}", new_caps_version, caps_locked.caps_version);
                    global_context_locked.caps = None;
                    global_context_locked.caps_last_attempted_ts = 0;
                }
            }
        }
    }
}

pub async fn create_global_context(
    cache_dir: PathBuf,
) -> (Arc<ARwLock<GlobalContext>>, std::sync::mpsc::Receiver<String>, CommandLine) {
    let cmdline = CommandLine::from_args();
    let (ask_shutdown_sender, ask_shutdown_receiver) = std::sync::mpsc::channel::<String>();
    let mut http_client_builder = reqwest::Client::builder();
    if cmdline.insecure {
        http_client_builder = http_client_builder.danger_accept_invalid_certs(true)
    }
    let http_client = http_client_builder.build().unwrap();


    let cx = GlobalContext {
        cmdline: cmdline.clone(),
        http_client: http_client,
        http_client_slowdown: Arc::new(Mutex::new(Slowdown { requests_in_flight: 0 })),
        cache_dir,
        caps: None,
        caps_last_attempted_ts: 0,
        tokenizer_map: HashMap::new(),
        completions_cache: Arc::new(StdRwLock::new(CompletionCache::new())),
        telemetry: Arc::new(StdRwLock::new(telemetry_structs::Storage::new())),
        vec_db: Arc::new(AMutex::new(None)),
        ask_shutdown_sender: Arc::new(Mutex::new(ask_shutdown_sender)),
        lsp_backend_document_state: LSPBackendDocumentState {
            document_map: Arc::new(ARwLock::new(HashMap::new())),
            workspace_folders: Arc::new(ARwLock::new(None)),
        },
    };
    (Arc::new(ARwLock::new(cx)), ask_shutdown_receiver, cmdline)
}
