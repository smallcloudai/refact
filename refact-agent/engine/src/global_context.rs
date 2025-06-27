use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;
use hyper::StatusCode;
use structopt::StructOpt;
use tokenizers::Tokenizer;
use tokio::signal;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock, Semaphore};
use tracing::{error, info};

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::caps::CodeAssistantCaps;
use crate::caps::providers::get_latest_provider_mtime;
use crate::completion_cache::CompletionCache;
use crate::custom_error::ScratchError;
use crate::files_in_workspace::DocumentsState;
use crate::integrations::docker::docker_ssh_tunnel_utils::SshTunnel;
use crate::integrations::sessions::IntegrationSession;
use crate::privacy::PrivacySettings;
use crate::background_tasks::BackgroundTasksHolder;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, default_value="pong", help="A message to return in /v1/ping, useful to verify you're talking to the same process that you've started.")]
    pub ping_message: String,
    #[structopt(long, help="Send logs to stderr, as opposed to ~/.cache/refact/logs, so it's easier to debug.")]
    pub logs_stderr: bool,
    #[structopt(long, default_value="", help="Send logs to a file.")]
    pub logs_to_file: String,
    #[structopt(long, short="u", default_value="", help="URL to use: \"Refact\" for Cloud, or your Self-Hosted Server URL. To bring your own keys, use \"Refact\" and set up providers.")]
    /// Inference server URL, or "Refact" for cloud
    pub address_url: String,
    #[structopt(long, short="k", default_value="", help="The API key to authenticate your requests, will appear in HTTP requests this binary makes.")]
    pub api_key: String,
    #[structopt(long, help="Trust self-signed SSL certificates, when connecting to an inference server.")]
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
    #[structopt(long, help="Wait until AST is ready before responding requests.")]
    pub wait_ast: bool,

    #[structopt(long, help="Use vector database. Give it LSP workspace folders or a jsonl, it also needs an embedding model.")]
    pub vecdb: bool,
    #[structopt(long, default_value="15000", help="Maximum files count for VecDB index, to avoid OOM.")]
    pub vecdb_max_files: usize,
    #[structopt(long, default_value="", help="Set VecDB storage path manually.")]
    pub vecdb_force_path: String,
    #[structopt(long, help="Wait until VecDB is ready before responding requests.")]
    pub wait_vecdb: bool,

    #[structopt(long, short="f", default_value="", help="A path to jsonl file with {\"path\": ...} on each line, files will immediately go to VecDB and AST.")]
    pub files_jsonl_path: String,
    #[structopt(long, short="w", default_value="", help="Workspace folder to find all the files. An LSP or HTTP request can override this later.")]
    pub workspace_folder: String,

    #[structopt(long, help="create yaml configs, like customization.yaml, privacy.yaml and exit.")]
    pub only_create_yaml_configs: bool,
    #[structopt(long, help="Print combined customization settings from both system defaults and customization.yaml.")]
    pub print_customization: bool,

    #[structopt(long, help="Enable experimental features, such as new integrations.")]
    pub experimental: bool,

    #[structopt(long, help="A way to tell this binary it can run more tools without confirmation.")]
    pub inside_container: bool,

    #[structopt(long, default_value="", help="Specify the integrations.yaml, this also disables the global integrations.d")]
    pub integrations_yaml: String,

    #[structopt(long, default_value="", help="Specify the variables.yaml, disabling the global one")]
    pub variables_yaml: String,
    #[structopt(long, default_value="", help="Specify the secrets.yaml, disabling the global one")]
    pub secrets_yaml: String,
    #[structopt(long, default_value="", help="Specify the indexing.yaml, replacing the global one")]
    pub indexing_yaml: String,
    #[structopt(long, default_value="", help="Specify the privacy.yaml, replacing the global one")]
    pub privacy_yaml: String,

    #[structopt(long, help="An pre-setup active group id")]
    pub active_group_id: Option<String>,
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
    pub shutdown_flag: Arc<AtomicBool>,
    pub cmdline: CommandLine,
    pub http_client: reqwest::Client,
    pub http_client_slowdown: Arc<Semaphore>,
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub caps: Option<Arc<CodeAssistantCaps>>,
    pub caps_reading_lock: Arc<AMutex<bool>>,
    pub caps_last_error: String,
    pub caps_last_attempted_ts: u64,
    pub tokenizer_map: HashMap<String, Option<Arc<Tokenizer>>>,
    pub tokenizer_download_lock: Arc<AMutex<bool>>,
    pub completions_cache: Arc<StdRwLock<CompletionCache>>,
    pub vec_db: Arc<AMutex<Option<crate::vecdb::vdb_highlev::VecDb>>>,
    pub vec_db_error: String,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub ask_shutdown_sender: Arc<StdMutex<std::sync::mpsc::Sender<String>>>,
    pub documents_state: DocumentsState,
    pub at_commands_preview_cache: Arc<AMutex<AtCommandsPreviewCache>>,
    pub privacy_settings: Arc<PrivacySettings>,
    pub indexing_everywhere: Arc<crate::files_blocklist::IndexingEverywhere>,
    pub integration_sessions: HashMap<String, Arc<AMutex<Box<dyn IntegrationSession>>>>,
    pub codelens_cache: Arc<AMutex<crate::http::routers::v1::code_lens::CodeLensCache>>,
    pub docker_ssh_tunnel: Arc<AMutex<Option<SshTunnel>>>,
    pub active_group_id: Option<String>,
    pub init_shadow_repos_background_task_holder: BackgroundTasksHolder,
    pub init_shadow_repos_lock: Arc<AMutex<bool>>,
    pub git_operations_abort_flag: Arc<AtomicBool>,
    pub app_searchable_id: String,
    pub threads_subscription_restart_flag: Arc<AtomicBool>
}

pub type SharedGlobalContext = Arc<ARwLock<GlobalContext>>;  // TODO: remove this type alias, confusing

const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds


pub async fn migrate_to_config_folder(
    config_dir: &PathBuf,
    cache_dir: &PathBuf
) -> io::Result<()> {
    let mut entries = tokio::fs::read_dir(cache_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let file_type = entry.file_type().await?;
        let is_yaml_cfg = file_type.is_file() && path.extension().and_then(|e| e.to_str()) == Some("yaml");
        if is_yaml_cfg {
            let new_path = config_dir.join(&file_name);
            if new_path.exists() {
                tracing::info!("cannot migrate {:?} to {:?}: destination exists", path, new_path);
                continue;
            }
            tokio::fs::rename(&path, &new_path).await?;
            tracing::info!("migrated {:?} to {:?}", path, new_path);
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn get_app_searchable_id(workspace_folders: &[PathBuf]) -> String {
    use std::process::Command;
    use rand::Rng;
    
    // Try multiple methods to get a unique machine identifier on macOS
    let machine_id = {
        // First attempt: Use system_profiler to get hardware UUID (most reliable)
        let hardware_uuid = Command::new("system_profiler")
            .args(&["SPHardwareDataType"])
            .output()
            .ok()
            .and_then(|output| {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Extract Hardware UUID from system_profiler output
                output_str.lines()
                    .find(|line| line.contains("Hardware UUID"))
                    .and_then(|line| {
                        line.split(':')
                            .nth(1)
                            .map(|s| s.trim().to_string())
                    })
            });
            
        if let Some(uuid) = hardware_uuid {
            if !uuid.trim().is_empty() {
                return uuid;
            }
        }
        
        // Second attempt: Try to get the serial number
        let serial_number = Command::new("system_profiler")
            .args(&["SPHardwareDataType"])
            .output()
            .ok()
            .and_then(|output| {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.lines()
                    .find(|line| line.contains("Serial Number"))
                    .and_then(|line| {
                        line.split(':')
                            .nth(1)
                            .map(|s| s.trim().to_string())
                    })
            });
            
        if let Some(serial) = serial_number {
            if !serial.trim().is_empty() {
                return serial;
            }
        }
        
        // Third attempt: Try to get the MAC address using ifconfig
        let mac_address = Command::new("ifconfig")
            .args(&["en0"])
            .output()
            .ok()
            .and_then(|output| {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.lines()
                    .find(|line| line.contains("ether"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .nth(1)
                            .map(|s| s.trim().replace(":", ""))
                    })
            });
            
        if let Some(mac) = mac_address {
            if !mac.trim().is_empty() && mac != "000000000000" {
                return mac;
            }
        }
        
        // Final fallback: Generate a random ID and store it persistently
        // This is just a temporary solution in case all other methods fail
        let mut rng = rand::thread_rng();
        format!("macos-{:016x}", rng.gen::<u64>())
    };

    let folders = workspace_folders
        .iter()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    format!("{}-{}", machine_id, folders)
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn get_app_searchable_id(workspace_folders: &[PathBuf]) -> String {
    let mac = pnet_datalink::interfaces()
        .into_iter()
        .find(|iface: &pnet_datalink::NetworkInterface| {
            !iface.is_loopback() && iface.mac.is_some()
        })
        .and_then(|iface| iface.mac)
        .map(|mac| mac.to_string().replace(":", ""))
        .unwrap_or_else(|| "no-mac".to_string());

    let folders = workspace_folders
        .iter()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    format!("{}-{}", mac, folders)
}

#[cfg(target_os = "windows")]
pub fn get_app_searchable_id(workspace_folders: &[PathBuf]) -> String {
    use winreg::enums::*;
    use winreg::RegKey;
    let machine_guid = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .and_then(|key| key.get_value::<String, _>("MachineGuid"))
        .unwrap_or_else(|_| "no-machine-guid".to_string());
    let folders = workspace_folders
        .iter()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");
    format!("{}-{}", machine_guid, folders)
}

pub async fn try_load_caps_quickly_if_not_present(
    gcx: Arc<ARwLock<GlobalContext>>,
    max_age_seconds: u64,
) -> Result<Arc<CodeAssistantCaps>, ScratchError> {
    let cmdline = CommandLine::from_args();  // XXX make it Arc and don't reload all the time
    let (caps_reading_lock, config_dir) = {
        let gcx_locked = gcx.read().await;
        (gcx_locked.caps_reading_lock.clone(), gcx_locked.config_dir.clone())
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let caps_last_attempted_ts;
    let latest_provider_mtime = get_latest_provider_mtime(&config_dir).await.unwrap_or(0);

    {
        // gcx is not locked, but a specialized async mutex is, up until caps are saved
        let _caps_reading_locked = caps_reading_lock.lock().await;

        let max_age = if max_age_seconds > 0 { max_age_seconds } else { CAPS_BACKGROUND_RELOAD };
        {
            let mut cx_locked = gcx.write().await;
            if cx_locked.caps_last_attempted_ts + max_age < now || latest_provider_mtime >= cx_locked.caps_last_attempted_ts {
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
                if caps.caps_version < new_caps_version {
                    info!("detected biggyback caps version {} is newer than the current version {}", new_caps_version, caps.caps_version);
                    gcx_locked.caps = None;
                    gcx_locked.caps_last_attempted_ts = 0;
                }
            }
        }
    }
}

pub async fn block_until_signal(
    ask_shutdown_receiver: std::sync::mpsc::Receiver<String>,
    shutdown_flag: Arc<AtomicBool>,
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
    config_dir: PathBuf,
) -> (Arc<ARwLock<GlobalContext>>, std::sync::mpsc::Receiver<String>, CommandLine) {
    let cmdline = CommandLine::from_args();
    let (ask_shutdown_sender, ask_shutdown_receiver) = std::sync::mpsc::channel::<String>();
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
        shutdown_flag: Arc::new(AtomicBool::new(false)),
        cmdline: cmdline.clone(),
        http_client,
        http_client_slowdown: Arc::new(Semaphore::new(2)),
        cache_dir,
        config_dir: config_dir.clone(),
        caps: None,
        caps_reading_lock: Arc::new(AMutex::<bool>::new(false)),
        caps_last_error: String::new(),
        caps_last_attempted_ts: 0,
        tokenizer_map: HashMap::new(),
        tokenizer_download_lock: Arc::new(AMutex::<bool>::new(false)),
        completions_cache: Arc::new(StdRwLock::new(CompletionCache::new())),
        vec_db: Arc::new(AMutex::new(None)),
        vec_db_error: String::new(),
        ast_service: None,
        ask_shutdown_sender: Arc::new(StdMutex::new(ask_shutdown_sender)),
        documents_state: DocumentsState::new(workspace_dirs.clone()).await,
        at_commands_preview_cache: Arc::new(AMutex::new(AtCommandsPreviewCache::new())),
        privacy_settings: Arc::new(PrivacySettings::default()),
        indexing_everywhere: Arc::new(crate::files_blocklist::IndexingEverywhere::default()),
        integration_sessions: HashMap::new(),
        codelens_cache: Arc::new(AMutex::new(crate::http::routers::v1::code_lens::CodeLensCache::default())),
        docker_ssh_tunnel: Arc::new(AMutex::new(None)),
        active_group_id: cmdline.active_group_id.clone(),
        init_shadow_repos_background_task_holder: BackgroundTasksHolder::new(vec![]),
        init_shadow_repos_lock: Arc::new(AMutex::new(false)),
        git_operations_abort_flag: Arc::new(AtomicBool::new(false)),
        app_searchable_id: get_app_searchable_id(&workspace_dirs),
        threads_subscription_restart_flag: Arc::new(AtomicBool::new(false)),
    };
    let gcx = Arc::new(ARwLock::new(cx));
    crate::files_in_workspace::watcher_init(gcx.clone()).await;
    (gcx, ask_shutdown_receiver, cmdline)
}
