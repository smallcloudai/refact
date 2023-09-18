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
    pub caps: Arc<StdRwLock<CodeAssistantCaps>>,
    pub cmdline: CommandLine,
}


const CAPS_RELOAD_EACH: u64 = 3600;  // seconds

pub async fn reload_caps(
    global_context: Arc<ARwLock<GlobalContext>>,
) -> () {
    let cmdline = CommandLine::from_args();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(CAPS_RELOAD_EACH)).await;
        let caps_result = crate::caps::load_recommendations(
            cmdline.clone()
        ).await;
        match caps_result {
            Ok(caps) => {
                let mut global_context_locked = global_context.write().await;
                global_context_locked.caps = caps;
                info!("reload caps successful");
            },
            Err(e) => {
                error!("failed to load caps: {}", e);
            }
        }
    }
}

pub async fn create_global_context(
    home_dir: PathBuf,
) -> Result<Arc<ARwLock<GlobalContext>>, String> {
    let cmdline = CommandLine::from_args();
    let caps = crate::caps::load_recommendations(
        cmdline.clone()
    ).await?;
    let cache_dir = home_dir.join(".cache/refact");
    info!("cache dir: {}", cache_dir.display());
    let cx = GlobalContext {
        http_client: reqwest::Client::new(),
        cache_dir,
        tokenizer_map: HashMap::new(),
        caps,
        cmdline,
    };
    Ok(Arc::new(ARwLock::new(cx)))
}
