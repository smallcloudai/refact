use tracing::{info, error};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;
use structopt::StructOpt;
use crate::caps::CodeAssistantCaps;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, short="u", help="URL to start working. The first step is to fetch coding_assistant_caps.json.")]
    pub address_url: String,
    #[structopt(long, short="p", default_value="8000", help="Bind 127.0.0.1:<port> to listen for requests.")]
    pub port: u16,
}


pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub caps: Option<Arc<StdRwLock<CodeAssistantCaps>>>,
    pub caps_last_attempted_ts: u64,
    pub cmdline: CommandLine,
}


const CAPS_RELOAD_BACKOFF: u64 = 60;       // seconds
const CAPS_BACKGROUND_RELOAD: u64 = 3600;  // seconds

pub async fn caps_background_reload(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    let cmdline = CommandLine::from_args();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(CAPS_BACKGROUND_RELOAD)).await;
        let caps_result = crate::caps::load_recommendations(
            cmdline.clone()
        ).await;
        match caps_result {
            Ok(caps) => {
                let mut global_context_locked = global_context.write().await;
                global_context_locked.caps = Some(caps);
                info!("background reload caps successful");
            },
            Err(e) => {
                error!("failed to load caps: {}", e);
            }
        }
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
                info!("reload caps successful");
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
    // let caps_result = crate::caps::load_recommendations(
    //     cmdline.clone()
    // ).await;
    // let caps_option = caps_result.ok().map_or(None, |x| Some(x));
    let cache_dir = home_dir.join(".cache/refact");
    info!("cache dir: {}", cache_dir.display());
    let cx = GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir,
        tokenizer_map: HashMap::new(),
        caps: None,
        caps_last_attempted_ts: 0,
        cmdline,
    };
    Ok(Arc::new(ARwLock::new(cx)))
}
