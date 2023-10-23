use tracing::{info, error};
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
use crate::telemetry_storage;
use crate::vecdb_search::VecdbSearch;
use crate::custom_error::ScratchError;
use hyper::StatusCode;


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
    #[structopt(long, default_value="0", help="Bind 127.0.0.1:<port> and act as an LSP server. This is compatible with having an HTTP server at the same time.")]
    pub lsp_port: u16,
    #[structopt(long, default_value="0", help="Act as an LSP server, use stdin stdout for communication. This is compatible with having an HTTP server at the same time. But it's not compatible with LSP port.")]
    pub lsp_stdin_stdout: u16,
}


// #[derive(Debug)]
pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub ask_shutdown_sender: Arc<Mutex<std::sync::mpsc::Sender<String>>>,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>,
    pub caps_last_attempted_ts: u64,
    pub cmdline: CommandLine,
    pub completions_cache: Arc<StdRwLock<CompletionCache>>,
    pub telemetry: Arc<StdRwLock<telemetry_storage::Storage>>,
    pub vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
}


const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn caps_background_reload(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    loop {
        let caps_result = crate::caps::load_caps(
            CommandLine::from_args()
        ).await;
        match caps_result {
            Ok(caps) => {
                let mut global_context_locked = global_context.write().await;
                global_context_locked.caps = Some(caps);
                info!("background reload caps successful");
                write!(std::io::stderr(), "CAPS\n").unwrap();
            },
            Err(e) => {
                error!("failed to load caps: {}", e);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(CAPS_BACKGROUND_RELOAD)).await;
    }
}

pub async fn try_load_caps_quickly_if_not_present(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, ScratchError> {
    let caps_last_attempted_ts;
    {
        let cx_locked = global_context.write().await;
        if let Some(caps_arc) = cx_locked.caps.clone() {
            return Ok(caps_arc.clone());
        }
        caps_last_attempted_ts = cx_locked.caps_last_attempted_ts;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if caps_last_attempted_ts + CAPS_RELOAD_BACKOFF > now {
        return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "server is not reachable, no caps available".to_string()));
    }
    let caps_result = crate::caps::load_caps(
        CommandLine::from_args()
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

pub async fn create_global_context(
    cache_dir: PathBuf,
) -> (Arc<ARwLock<GlobalContext>>, std::sync::mpsc::Receiver<String>, CommandLine) {
    let cmdline = CommandLine::from_args();
    let (ask_shutdown_sender, ask_shutdown_receiver) = std::sync::mpsc::channel::<String>();
    let cx = GlobalContext {
        http_client: reqwest::Client::new(),
        ask_shutdown_sender: Arc::new(Mutex::new(ask_shutdown_sender)),
        cache_dir,
        tokenizer_map: HashMap::new(),
        caps: None,
        caps_last_attempted_ts: 0,
        cmdline: cmdline.clone(),
        completions_cache: Arc::new(StdRwLock::new(CompletionCache::new())),
        telemetry: Arc::new(StdRwLock::new(telemetry_storage::Storage::new())),
        vecdb_search: Arc::new(AMutex::new(Box::new(crate::vecdb_search::VecdbSearchTest::new()))),
    };
    (Arc::new(ARwLock::new(cx)), ask_shutdown_receiver, cmdline)
}
