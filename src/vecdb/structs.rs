use std::fmt::Debug;
use std::path::PathBuf;
use std::time::SystemTime;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};


#[async_trait]
pub trait VecdbSearch: Send {
    async fn search(
        &self,
        query: String,
        top_n: usize,
    ) -> Result<SearchResult, String>;
}

#[derive(Debug, Clone)]
pub struct VecdbConstants {
    // constant in a sense it cannot be changed without creating a new db
    pub model_name: String,
    pub embedding_size: i32,
    pub endpoint_embeddings_template: String,
    pub endpoint_embeddings_style: String,
    pub cooldown_secs: u64,
    pub splitter_window_size: usize,
    pub splitter_soft_limit: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VecDbStatus {
    pub unprocessed_files_count: usize,
    pub requests_made_since_start: usize,
    pub db_size: usize,
    pub db_cache_size: usize,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query_text: String,
    pub results: Vec<Record>,
}
