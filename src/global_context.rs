use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;
use hyper::StatusCode;
use structopt::StructOpt;
use tokenizers::Tokenizer;
use tokio::signal;
use tokio::sync::{Mutex as AMutex, Semaphore};
use tokio::sync::RwLock as ARwLock;
use tracing::{error, info};

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::caps::CodeAssistantCaps;
use crate::completion_cache::CompletionCache;
use crate::custom_error::ScratchError;
use crate::files_in_workspace::DocumentsState;
use crate::integrations::sessions::IntegrationSession;
use crate::privacy::PrivacySettings;
use crate::telemetry::telemetry_structs;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, default_value="pong", help="A message to return in /v1/ping, useful to verify you're talking to the same process that you've started.")]
    pub ping_message: String,
    #[structopt(long, help="Send logs to stderr, as opposed to ~/.cache/refact/logs, so it's easier to debug.")]
    pub logs_stderr: bool,
    #[structopt(long, default_value="", help="Send logs to a file.")]
    pub logs_to_file: String,
    #[structopt(long, short="u", default_value="", help="URL to start working. The first step is to fetch capabilities from $URL/refact-caps. You can supply your own caps in a local file, too, for the bring-your-own-key use case.")]
    pub address_url: String,
    #[structopt(long, short="k", default_value="", help="The API key to authenticate your requests, will appear in HTTP requests this binary makes.")]
    pub api_key: String,
    #[structopt(long, help="Trust self-signed SSL certificates")]
    pub insecure: bool,

    #[structopt(long, short="p", default_value="0", help="Bind 127.0.0.1:<port> to listen for HTTP requests, such as /v1/code-completion, /v1/chat, /v1/caps.")]
    pub http_port: u16,
    #[structopt(long, default_value="0", help="Bind 127.0.0.1:<port> and act as an LSP server. This is compatible with having an HTTP server at the same time.")]
    pub lsp_port: u16,
    #[structopt(long, default_value="0", help="Act as an LSP server, use stdin stdout for communication. This is compatible with having an HTTP server at the same time. But it's not compatible with LSP port.")]
    pub lsp_stdin_stdout: u16,

    #[structopt(long, default_value="", help="End-user client version, such as version of VS Code plugin.")]
    pub enduser_client_version: String,
    #[structopt(long, short="b", help="Send basic telemetry (counters and errors).")]
    pub basic_telemetry: bool,
    #[structopt(long, short="v", help="Makes DEBUG log level visible, instead of the default INFO.")]
    pub verbose: bool,

    #[structopt(long, help="Use AST, for it to start working, give it a jsonl files list or LSP workspace folders.")]
    pub ast: bool,
    // #[structopt(long, help="Use AST light mode, could be useful for large projects and little memory. Less information gets stored.")]
    // pub ast_light_mode: bool,
    #[structopt(long, default_value="50000", help="Maximum files for AST index, to avoid OOM on large projects.")]
    pub ast_max_files: usize,
    #[structopt(long, default_value="", help="Give it a path for AST database to make it permanent, if there is the database already, process starts without parsing all the files (careful). This quick start is helpful for automated solution search.")]
    pub ast_permanent: String,

    #[cfg(feature="vecdb")]
    #[structopt(long, help="Use vector database. Give it LSP workspace folders or a jsonl, it also needs an embedding model.")]
    pub vecdb: bool,
    #[cfg(feature="vecdb")]
    #[structopt(long, help="Delete all memories, start with empty memory.")]
    pub reset_memory: bool,
    #[cfg(feature="vecdb")]
    #[structopt(long, default_value="15000", help="Maximum files count for VecDB index, to avoid OOM.")]
    pub vecdb_max_files: usize,
    #[cfg(feature="vecdb")]
    #[structopt(long, default_value="", help="Set VecDB storage path manually.")]
    pub vecdb_force_path: String,

    #[structopt(long, short="f", default_value="", help="A path to jsonl file with {\"path\": ...} on each line, files will immediately go to VecDB and AST.")]
    pub files_jsonl_path: String,
    #[structopt(long, short="w", default_value="", help="Workspace folder to find all the files. An LSP or HTTP request can override this later.")]
    pub workspace_folder: String,

    #[structopt(long, help="create manually bring-your-own-key.yaml, integrations.yaml, customization.yaml and privacy.yaml and EXIT")]
    pub only_create_yaml_configs: bool,

    #[structopt(long, help="Enable experimental features, such as new integrations.")]
    pub experimental: bool,
}

impl CommandLine {
    fn create_hash(msg: String) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write(msg.as_bytes());
        format!("{:x}", hasher.finish())
    }

    pub fn get_prefix(&self) -> String {
        // This helps several self-hosting or cloud accounts to not mix
        Self::create_hash(format!("{}:{}", self.address_url.clone(), self.api_key.clone()))[..6].to_string()
    }
}

pub struct AtCommandsPreviewCache {
    pub cache: HashMap<String, String>,
}

impl AtCommandsPreviewCache {
    pub fn new() -> Self { Self { cache: HashMap::new() } }
    pub fn get(&self, key: &str) -> Option<String> {
        let val = self.cache.get(key).cloned();
        // if val.is_some() {
        //     info!("AtCommandsPreviewCache: SOME: key={:?}", key);
        // } else {
        //     info!("AtCommandsPreviewCache: NONE: key={:?}", key);
        // }
        val
    }
    pub fn insert(&mut self, key: String, value: String) {
        self.cache.insert(key.clone(), value);
        // info!("AtCommandsPreviewCache: insert: key={:?}. new_len: {:?}", key, self.cache.len());
    }
    pub fn clear(&mut self) {
        self.cache.clear();
        // info!("AtCommandsPreviewCache: clear; new_len: {:?}", self.cache.len());
    }
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
    #[cfg(feature="vecdb")]
    pub vec_db: Arc<AMutex<Option<crate::vecdb::vdb_highlev::VecDb>>>,
    #[cfg(not(feature="vecdb"))]
    pub vec_db: bool,
    pub vec_db_error: String,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub ask_shutdown_sender: Arc<StdMutex<std::sync::mpsc::Sender<String>>>,
    pub documents_state: DocumentsState,
    pub at_commands_preview_cache: Arc<AMutex<AtCommandsPreviewCache>>,
    pub privacy_settings: Arc<PrivacySettings>,
    pub integration_sessions: HashMap<String, Arc<AMutex<Box<dyn IntegrationSession>>>>,
}

pub type SharedGlobalContext = Arc<ARwLock<GlobalContext>>;  // TODO: remove this type alias, confusing

const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn try_load_caps_quickly_if_not_present(
    gcx: Arc<ARwLock<GlobalContext>>,
    max_age_seconds: u64,
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, ScratchError> {
    let cmdline = CommandLine::from_args();  // XXX make it Arc and don't reload all the time

    let caps_reading_lock: Arc<AMutex<bool>> = gcx.read().await.caps_reading_lock.clone();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let caps_last_attempted_ts;
    {
        // gcx is not locked, but a specialized async mutex is, up until caps are saved
        let _caps_reading_locked = caps_reading_lock.lock().await;

        let caps_url = cmdline.address_url.clone();
        if caps_url.to_lowercase() == "refact" || caps_url.starts_with("http") {
            let max_age = if max_age_seconds > 0 { max_age_seconds } else { CAPS_BACKGROUND_RELOAD };
            {
                let mut cx_locked = gcx.write().await;
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
                let gcx_locked = gcx.write().await;
                return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, gcx_locked.caps_last_error.clone()));
            }
        }

        let caps_result = crate::caps::load_caps(
            cmdline,
            gcx.clone()
        ).await;

        {
            let mut gcx_locked = gcx.write().await;
            gcx_locked.caps_last_attempted_ts = now;
            match caps_result {
                Ok(caps) => {
                    gcx_locked.caps = Some(caps.clone());
                    gcx_locked.caps_last_error = "".to_string();
                    info!("quick load caps successful");
                    let _ = write!(std::io::stderr(), "CAPS\n");
                    Ok(caps)
                },
                Err(e) => {
                    error!("caps fetch failed: {:?}", e);
                    gcx_locked.caps_last_error = format!("caps fetch failed: {}", e);
                    return Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, gcx_locked.caps_last_error.clone()));
                }
            }
        }
    }
}

pub async fn look_for_piggyback_fields(
    gcx: Arc<ARwLock<GlobalContext>>,
    anything_from_server: &serde_json::Value)
{
    let mut gcx_locked = gcx.write().await;
    if let Some(dict) = anything_from_server.as_object() {
        let new_caps_version = dict.get("caps_version").and_then(|v| v.as_i64()).unwrap_or(0);
        if new_caps_version > 0 {
            if let Some(caps) = gcx_locked.caps.clone() {
                let caps_locked = caps.read().unwrap();
                if caps_locked.caps_version < new_caps_version {
                    info!("detected biggyback caps version {} is newer than the current version {}", new_caps_version, caps_locked.caps_version);
                    gcx_locked.caps = None;
                    gcx_locked.caps_last_attempted_ts = 0;
                }
            }
        }
    }
}

pub async fn block_until_signal(
    ask_shutdown_receiver: std::sync::mpsc::Receiver<String>,
    shutdown_flag: Arc<AtomicBool>
) {
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

    let shutdown_flag_clone = shutdown_flag.clone();
    tokio::select! {
        _ = ctrl_c => {
            info!("SIGINT signal received");
            shutdown_flag_clone.store(true, Ordering::SeqCst);
        },
        _ = sigterm => {
            info!("SIGTERM signal received");
            shutdown_flag_clone.store(true, Ordering::SeqCst);
        },
        _ = sigusr1 => {
            info!("SIGUSR1 signal received");
        },
        _ = tokio::task::spawn_blocking(move || {
            let _ = ask_shutdown_receiver.recv();
            shutdown_flag.store(true, Ordering::SeqCst);
        }) => {
            info!("graceful shutdown to store telemetry");
        }
    }
}

pub async fn create_global_context(
    cache_dir: PathBuf,
) -> (Arc<ARwLock<GlobalContext>>, std::sync::mpsc::Receiver<String>, Arc<AtomicBool>, CommandLine) {
    let cmdline = CommandLine::from_args();
    let (ask_shutdown_sender, ask_shutdown_receiver) = std::sync::mpsc::channel::<String>();
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let mut http_client_builder = reqwest::Client::builder();
    if cmdline.insecure {
        http_client_builder = http_client_builder.danger_accept_invalid_certs(true)
    }
    let http_client = http_client_builder.build().unwrap();

    let mut workspace_dirs: Vec<PathBuf> = vec![];
    if !cmdline.workspace_folder.is_empty() {
        let path = crate::files_correction::canonical_path(&cmdline.workspace_folder);
        workspace_dirs = vec![path];
    }
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
        #[cfg(feature="vecdb")]
        vec_db: Arc::new(AMutex::new(None)),
        #[cfg(not(feature="vecdb"))]
        vec_db: false,
        vec_db_error: String::new(),
        ast_service: None,
        ask_shutdown_sender: Arc::new(StdMutex::new(ask_shutdown_sender)),
        documents_state: DocumentsState::new(workspace_dirs).await,
        at_commands_preview_cache: Arc::new(AMutex::new(AtCommandsPreviewCache::new())),
        privacy_settings: Arc::new(PrivacySettings::default()),
        integration_sessions: HashMap::new(),
    };
    let gcx = Arc::new(ARwLock::new(cx));
    {
        let gcx_weak = Arc::downgrade(&gcx);
        gcx.write().await.documents_state.init_watcher(gcx_weak);
    }
    (gcx, ask_shutdown_receiver, shutdown_flag, cmdline)
}