use std::fmt::Debug;
use std::path::PathBuf;
use std::time::SystemTime;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::RwLock as StdRwLock;
use tokenizers::Tokenizer;
use std::sync::Arc;


#[async_trait]
pub trait VecdbSearch: Send {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        filter_mb: Option<String>,
    ) -> Result<SearchResult, String>;
}

#[derive(Debug, Clone)]
pub struct VecdbConstants {
    // constant in a sense it cannot be changed without creating a new db
    pub model_name: String,
    pub embedding_size: i32,
    pub tokenizer: Arc<StdRwLock<Tokenizer>>,
    pub vectorizer_n_ctx: usize,
    pub endpoint_embeddings_template: String,
    pub endpoint_embeddings_style: String,
    pub cooldown_secs: u64,
    pub splitter_window_size: usize,
    pub splitter_soft_limit: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VecDbStatus {
    pub files_unprocessed: usize,
    pub files_total: usize,  // only valid for status bar in the UI, resets to 0 when done
    pub requests_made_since_start: usize,
    pub vectors_made_since_start: usize,
    pub db_size: usize,
    pub db_cache_size: usize,
    pub state: String
}


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Record {
    pub vector: Option<Vec<f32>>,
    pub window_text: String,
    pub window_text_hash: String,
    pub file_path: PathBuf,
    pub start_line: u64,
    pub end_line: u64,
    pub time_added: SystemTime,
    pub time_last_used: SystemTime,
    pub model_name: String,
    pub used_counter: u64,
    pub distance: f32,
    pub usefulness: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct SplitResult {
    pub file_path: PathBuf,
    pub window_text: String,
    pub window_text_hash: String,
    pub start_line: u64,
    pub end_line: u64,
    pub symbol_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query_text: String,
    pub results: Vec<Record>,
}
