use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;

use hyper::StatusCode;
use structopt::StructOpt;
use tokenizers::Tokenizer;
use tokio::signal;
use tokio::sync::{Mutex as AMutex, Semaphore};
use tokio::sync::RwLock as ARwLock;
use tracing::{error, info};
use url::Url;

use crate::ast::ast_module::AstModule;
use crate::caps::CodeAssistantCaps;
use crate::completion_cache::CompletionCache;
use crate::custom_error::ScratchError;
use crate::files_in_workspace::Document;
use crate::telemetry::telemetry_structs;
use crate::vecdb::vecdb::VecDb;

#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, help="Send logs to stderr, as opposed to ~/.cache/refact/logs, so it's easier to debug.")]
    pub logs_stderr: bool,
    #[structopt(long, short="u", help="URL to start working. The first step is to fetch refact-caps / coding_assistant_caps.json.")]
    pub address_url: String,
    #[structopt(long, short="k", default_value="", help="The API key to authenticate your requests, will appear in HTTP requests this binary makes.")]
    pub api_key: String,
    #[structopt(long, short="p", default_value="0", help="Bind 127.0.0.1:<port> to listen for HTTP requests, such as /v1/code-completion, /v1/chat, /v1/caps.")]
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
    #[structopt(long, short="v", help="Verbose logging, lots of output")]
    pub verbose: bool,
    #[structopt(long, help="Use AST. For it to start working, give it a jsonl files list or LSP workspace folders.")]
    pub ast: bool,
    #[structopt(long, help="Use vector database. Give it a jsonl files list or LSP workspace folders, and also caps need to have an embedding model.")]
    pub vecdb: bool,
    #[structopt(long, short="f", default_value="", help="A path to jsonl file with {\"path\": ...} on each line, files will immediately go to vecdb and ast")]
    pub files_jsonl_path: String,
    #[structopt(long, default_value="", help="Vecdb storage path")]
    pub vecdb_forced_path: String,
    #[structopt(long, short="w", default_value="", help="Workspace folder to find files for vecdb and AST. An LSP or HTTP request can override this later.")]
    pub workspace_folder: String,
}
impl CommandLine {
    fn create_hash(msg: String) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write(msg.as_bytes());
        format!("{:x}", hasher.finish())
    }
    pub fn get_prefix(&self) -> String {
        Self::create_hash(format!("{}:{}", self.address_url.clone(), self.api_key.clone()))[..6].to_string()
    }
}

pub struct DocumentsState {
    pub workspace_folders: Arc<StdMutex<Vec<PathBuf>>>,
    pub workspace_files: Arc<StdMutex<Vec<Url>>>,
    pub document_map: Arc<ARwLock<HashMap<Url, Document>>>,   // if a file is open in IDE and it's outside workspace dirs, it will be in this map and not in workspace_files
    pub cache_dirty: Arc<AMutex<bool>>,
    pub cache_correction: Arc<HashMap<String, String>>,  // map dir3/file.ext -> to /dir1/dir2/dir3/file.ext
    pub cache_fuzzy: Arc<Vec<String>>,                   // slow linear search
}

pub struct GlobalContext {
    pub cmdline: CommandLine,
    pub http_client: reqwest::Client,
    pub http_client_slowdown: Arc<Semaphore>,
    pub cache_dir: PathBuf,
    pub caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>,
    pub caps_reading_lock: Arc<AMutex<bool>>,
    pub caps_last_error: String,
    pub caps_last_attempted_ts: u64,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub tokenizer_download_lock: Arc<AMutex<bool>>,
    pub completions_cache: Arc<StdRwLock<CompletionCache>>,
    pub telemetry: Arc<StdRwLock<telemetry_structs::Storage>>,
    pub vec_db: Arc<AMutex<Option<VecDb>>>,
    pub ast_module: Arc<AMutex<Option<AstModule>>>,   // TODO: don't use AMutex, use StdMutex
    pub ask_shutdown_sender: Arc<StdMutex<std::sync::mpsc::Sender<String>>>,
    pub documents_state: DocumentsState,
}

pub type SharedGlobalContext = Arc<ARwLock<GlobalContext>>;  // TODO: remove this type alias, confusing

const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn try_load_caps_quickly_if_not_present(
    global_context: Arc<ARwLock<GlobalContext>>,
    max_age_seconds: u64,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, ScratchError> {
    let caps_reading_lock: Arc<AMutex<bool>> = global_context.read().await.caps_reading_lock.clone();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let caps_last_attempted_ts;
    {
        // global_context is not locked, but a specialized async mutex is, up until caps are saved
        let _caps_reading_locked = caps_reading_lock.lock().await;
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
            let global_context_locked = global_context.write().await;
            return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, global_context_locked.caps_last_error.clone()));
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
                    global_context_locked.caps_last_error = "".to_string();
                    info!("quick load caps successful");
                    write!(std::io::stderr(), "CAPS\n").unwrap();
                    Ok(caps)
                },
                Err(e) => {
                    error!("caps fetch failed: \"{}\"", e);
                    global_context_locked.caps_last_error = format!("caps fetch failed: {}", e);
                    return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, global_context_locked.caps_last_error.clone()));
                }
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

pub async fn block_until_signal(ask_shutdown_receiver: std::sync::mpsc::Receiver<String>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    #[cfg(unix)]
    let sigusr1 = async {
        signal::unix::signal(signal::unix::SignalKind::user_defined1())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let sigusr1 = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("SIGINT signal received");
        },
        _ = sigterm => {
            info!("SIGTERM signal received");
        },
        _ = sigusr1 => {
            info!("SIGUSR1 signal received");
        },
        _ = tokio::task::spawn_blocking(move || ask_shutdown_receiver.recv()) => {
            info!("graceful shutdown to store telemetry");
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
        http_client,
        http_client_slowdown: Arc::new(Semaphore::new(2)),
        cache_dir,
        caps: None,
        caps_reading_lock: Arc::new(AMutex::<bool>::new(false)),
        caps_last_error: String::new(),
        caps_last_attempted_ts: 0,
        tokenizer_map: HashMap::new(),
        tokenizer_download_lock: Arc::new(AMutex::<bool>::new(false)),
        completions_cache: Arc::new(StdRwLock::new(CompletionCache::new())),
        telemetry: Arc::new(StdRwLock::new(telemetry_structs::Storage::new())),
        vec_db: Arc::new(AMutex::new(None)),
        ast_module: Arc::new(AMutex::new(None)),
        ask_shutdown_sender: Arc::new(StdMutex::new(ask_shutdown_sender)),
        documents_state: DocumentsState {
            workspace_folders: if cmdline.workspace_folder.is_empty() { Arc::new(StdMutex::new(vec![])) } else { Arc::new(StdMutex::new(vec![PathBuf::from(cmdline.workspace_folder.clone())])) },
            workspace_files: Arc::new(StdMutex::new(vec![])),
            document_map: Arc::new(ARwLock::new(HashMap::new())),
            cache_dirty: Arc::new(AMutex::<bool>::new(false)),
            cache_correction: Arc::new(HashMap::<String, String>::new()),
            cache_fuzzy: Arc::new(Vec::<String>::new()),
        },
    };
    let gcx = Arc::new(ARwLock::new(cx));
    if cmdline.ast {
        let ast_module = Arc::new(AMutex::new(Some(
            AstModule::ast_indexer_init(gcx.clone()).await.expect("Failed to initialize ast module")
        )));
        gcx.write().await.ast_module = ast_module;
    }
    (gcx, ask_shutdown_receiver, cmdline)
}
