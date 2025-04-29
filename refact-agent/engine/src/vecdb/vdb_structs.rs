use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::RwLock as StdRwLock;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use tokenizers::Tokenizer;
use async_trait::async_trait;


#[async_trait]
pub trait VecdbSearch: Send {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        filter_mb: Option<String>,
        api_key: &String,
    ) -> Result<SearchResult, String>;
}

#[derive(Debug, Clone)]
pub struct VecdbConstants {
    // constant in a sense it cannot be changed without creating a new db
    pub embedding_model: String,
    pub embedding_size: i32,
    pub embedding_batch: usize,
    pub tokenizer: Option<Arc<StdRwLock<Tokenizer>>>,
    pub vectorizer_n_ctx: usize,
    pub endpoint_embeddings_template: String,
    pub endpoint_embeddings_style: String,
    pub splitter_window_size: usize,
    pub vecdb_max_files: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VecDbStatus {
    pub files_unprocessed: usize,
    pub files_total: usize,  // only valid for status bar in the UI, resets to 0 when done
    pub requests_made_since_start: usize,
    pub vectors_made_since_start: usize,
    pub db_size: usize,
    pub db_cache_size: usize,
    pub state: String,   // "starting", "parsing", "done", "cooldown"
    pub queue_additions: bool,
    pub vecdb_max_files_hit: bool,
    pub vecdb_errors: IndexMap<String, usize>,
}


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct VecdbRecord {
    pub vector: Option<Vec<f32>>,
    pub file_path: PathBuf,
    pub start_line: u64,
    pub end_line: u64,
    pub distance: f32,
    pub usefulness: f32,
}

#[derive(Debug, Clone)]
pub struct SplitResult {
    pub file_path: PathBuf,
    pub window_text: String,
    pub window_text_hash: String,
    pub start_line: u64,
    pub end_line: u64,
    pub symbol_path: String,
}

#[derive(Clone)]
pub struct SimpleTextHashVector {
    pub window_text: String,
    pub window_text_hash: String,
    pub vector: Option<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query_text: String,
    pub results: Vec<VecdbRecord>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MemoRecord {
    pub memid: String,
    pub thevec: Option<Vec<f32>>,
    pub distance: f32,
    pub m_type: String,
    pub m_goal: String,
    pub m_project: String,
    pub m_payload: String,
    pub m_origin: String,
    pub mstat_correct: f64,
    pub mstat_relevant: f64,
    pub mstat_times_used: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoSearchResult {
    pub query_text: String,
    pub results: Vec<MemoRecord>,
}
