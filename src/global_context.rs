use tracing::{info, error};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;
use structopt::StructOpt;
use std::io::Write;
use crate::caps::CodeAssistantCaps;
use crate::telemetry_basic;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, short="u", help="URL to start working. The first step is to fetch coding_assistant_caps.json.")]
    pub address_url: String,
    #[structopt(long, short="k", default_value="", help="API key used to authenticate your requests, will appear in HTTP requests this binary makes.")]
    pub api_key: String,
    #[structopt(long, short="p", default_value="8001", help="Bind 127.0.0.1:<port> to listen for requests, such as /v1/code-completion, /v1/chat, /v1/caps.")]
    pub port: u16,
    #[structopt(long, short="v", default_value="", help="End-user client version, such as version of VS Code plugin.")]
    pub enduser_client_version: String,
    #[structopt(long, short="b", help="Send basic telemetry (counters and errors)")]
    pub basic_telemetry: bool,
}


pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>,
    pub caps_last_attempted_ts: u64,
    pub cmdline: CommandLine,
    pub telemetry: Arc<StdRwLock<telemetry_basic::Storage>>,
}


const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn caps_background_reload(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    loop {
        let caps_result = crate::caps::load_recommendations(
            CommandLine::from_args()
        ).await;
        match caps_result {
            Ok(caps) => {
                let mut global_context_locked = global_context.write().await;
                global_context_locked.caps = Some(caps);
                info!("background reload caps successful");
                write!(std::io::stdout(), "CAPS\n").unwrap();
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
) -> Result<Arc<StdRwLock<CodeAssistantCaps>>, String> {
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
        return Err("server is not reachable, no caps available".to_string());
    }
    let caps_result = crate::caps::load_recommendations(
        CommandLine::from_args()
    ).await;
    {
        let mut global_context_locked = global_context.write().await;
        global_context_locked.caps_last_attempted_ts = now;
        match caps_result {
            Ok(caps) => {
                global_context_locked.caps = Some(caps.clone());
                info!("quick load caps successful");
                write!(std::io::stdout(), "CAPS\n").unwrap();
                Ok(caps)
            },
            Err(e) => {
                Err(format!("server is not reachable: {}", e))
            }
        }
    }
}

pub async fn create_global_context(
    home_dir: PathBuf,
) -> Result<Arc<ARwLock<GlobalContext>>, String> {
    let cmdline = CommandLine::from_args();
    let cache_dir = home_dir.join(".cache/refact");
    info!("cache dir: {}", cache_dir.display());
    let cx = GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir,
        tokenizer_map: HashMap::new(),
        caps: None,
        caps_last_attempted_ts: 0,
        cmdline,
        telemetry: Arc::new(StdRwLock::new(telemetry_basic::Storage::new())),
    };
    Ok(Arc::new(ARwLock::new(cx)))
}
