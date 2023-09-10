use tracing::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as ARwLock;
use tokenizers::Tokenizer;
use structopt::StructOpt;
use crate::recommendations::CodeAssistantRecommendations;


#[derive(Debug, StructOpt, Clone)]
pub struct CommandLine {
    #[structopt(long, short="u", help="URL to start working. The first step is to fetch coding_assistant_caps.json.")]
    pub address_url: String,
}


pub struct GlobalContext {
    pub http_client: reqwest::Client,
    pub cache_dir: PathBuf,
    pub tokenizer_map: HashMap< String, Arc<StdRwLock<Tokenizer>>>,
    pub caps: Arc<StdRwLock<CodeAssistantRecommendations>>,
    pub cmdline: CommandLine,
}

pub async fn create_global_context(
    home_dir: PathBuf,
) -> Result<Arc<ARwLock<GlobalContext>>, String> {
    let cmdline = CommandLine::from_args();
    let caps = crate::recommendations::load_recommendations(
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
