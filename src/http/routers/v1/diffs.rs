use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use hashbrown::HashMap;
use std::sync::Arc;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use tokio::sync::RwLock as ARwLock;
use crate::at_commands::at_file::file_repair_candidates;
use crate::call_validation::DiffChunk;
use crate::custom_error::ScratchError;
use crate::diffs::read_files_from_disk_and_patch;
use crate::global_context::GlobalContext;


const MAX_FUZZY_N: usize = 10;


#[derive(Deserialize)]
pub struct DiffPost {
    pub apply: Vec<bool>,
    pub chunks: Vec<DiffChunk>,
    #[serde(skip_serializing, default)]
    pub id: u64
}

impl DiffPost {
    pub fn set_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.chunks.hash(&mut hasher);
        self.id = hasher.finish();
    }
}

#[derive(Serialize)]
struct DiffResponseItem {
    chunk_id: usize,
    fuzzy_n_used: usize,
}

#[derive(Serialize)]
struct HandleDiffResponse {
    fuzzy_results: Vec<DiffResponseItem>,
    state: Vec<usize>,
}

pub async fn write_to_file(path: &String, text: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .await
        .map_err(|e| {
            format!("Failed to open file: {}", e)
        })?;

    file.write_all(text.as_bytes()).await.map_err(|e| {
        format!("Failed to write to file: {}", e)
    })?;
    Ok(())
}

fn results_into_state_vector(results: &HashMap<usize, Option<usize>>, total: usize) -> Vec<usize> {
    let mut state_vector = vec![0; total];
    for (k, v) in results {
        if *k < total {
            state_vector[*k] = if v.is_some() { 1 } else { 2 };
        }
    }
    state_vector
}

fn validate_post(post: &DiffPost) -> Result<(), ScratchError> {
    if post.chunks.is_empty() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` shouldn't be empty".to_string()));
    }
    if post.chunks.len() != post.apply.len() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` and `apply` arrays are not of the same length".to_string()));
    }
    Ok(())
}

fn validate_chunk(chunk: &DiffChunk) -> Result<(), String> {
    if chunk.line1 < 1 {
        return Err("Invalid line range: line1 cannot be < 1".to_string());
    }
    Ok(())
}

async fn init_chunks(
    chunks: &mut Vec<DiffChunk>, 
    apply: &Vec<bool>, 
    global_context: Arc<ARwLock<GlobalContext>>
) -> Result<(), ScratchError> {
    for ((c_idx, c), a) in chunks.iter_mut().enumerate().zip(apply.iter()) {
        c.chunk_id = c_idx;
        c.apply = *a;

        let file_path = PathBuf::from(&c.file_name);
        if !file_path.is_file() {
            let candidates = file_repair_candidates(&c.file_name, global_context.clone(), 5, false).await;
            let fuzzy_candidates = file_repair_candidates(&c.file_name, global_context.clone(), 5, true).await;

            if candidates.len() > 1 {
                return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("file_name `{}` is ambiguous.\nIt could be interpreted as:\n{}", &c.file_name, candidates.join("\n"))));
            }
            if candidates.is_empty() {
                return if !fuzzy_candidates.is_empty() {
                    Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("file_name `{}` is not found.\nHowever, there are similar paths:\n{}", &c.file_name, fuzzy_candidates.join("\n"))))
                } else {
                    Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("file_name `{}` is not found", &c.file_name)))
                }
            }
            let candidate = candidates.get(0).unwrap();
            if !PathBuf::from(&candidate).is_file() {
                return Err(ScratchError::new(StatusCode::BAD_REQUEST, format!("file_name `{}` is not found.\nHowever, there are similar paths:\n{}", &c.file_name, fuzzy_candidates.join("\n"))));
            }
            c.file_name = candidate.clone();
        }
        
        validate_chunk(c).map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, format!("error validating chunk {}:\n{}", c_idx, e)))?;
    }
    Ok(())
}

pub async fn handle_v1_diff_apply(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    validate_post(&post)?;
    init_chunks(&mut post.chunks, &post.apply, global_context.clone()).await?;

    // undo all chunks that are already applied to file, then apply chunks marked in post.apply
    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default() 
    };
    let mut chunks_undo = post.chunks.iter().filter(|x|applied_state.get(x.chunk_id) == Some(&1)).cloned().collect::<Vec<_>>();
    chunks_undo.iter_mut().for_each(|x|x.apply = true);
    let (texts_after_patch, fuzzy_n_used) = read_files_from_disk_and_patch(&post.chunks, &chunks_undo, MAX_FUZZY_N).await;
    
    for (file_name, new_text) in texts_after_patch.iter() {
        write_to_file(file_name, new_text).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    }

    let new_state = results_into_state_vector(&fuzzy_n_used, post.chunks.len());
    global_context.write().await.documents_state.diffs_applied_state.insert(post.id, new_state.clone());
    
    let fuzzy_results: Vec<DiffResponseItem> = fuzzy_n_used.iter().filter(|x|x.1.is_some())
        .map(|(chunk_id, fuzzy_n_used)| DiffResponseItem {
            chunk_id: chunk_id.clone(),
            fuzzy_n_used: fuzzy_n_used.unwrap()
        })
        .collect();

    let response = HandleDiffResponse {
        fuzzy_results,
        state: new_state,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(&response).unwrap()))
        .unwrap())
}

#[derive(Deserialize)]
struct DiffStatePost {
    pub chunks: Vec<DiffChunk>,
    #[serde(skip_serializing, default)]
    pub id: u64,
}

impl DiffStatePost {
    pub fn set_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.chunks.hash(&mut hasher);
        self.id = hasher.finish();
    }
}

#[derive(Serialize)]
struct DiffStateResponse {
    id: u64,
    state: Vec<usize>,
    can_apply: Vec<bool>,
}

pub async fn handle_v1_diff_state(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffStatePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    let apply = vec![true; post.chunks.len()];
    init_chunks(&mut post.chunks, &apply, global_context.clone()).await?;
    
    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default()
    };
    let chunks_undo = post.chunks.iter().filter(|x|applied_state.get(x.chunk_id) == Some(&1)).cloned().collect::<Vec<_>>();
    let (_, fuzzy_n_used) = read_files_from_disk_and_patch(&post.chunks, &chunks_undo, MAX_FUZZY_N).await;
    let new_state = results_into_state_vector(&fuzzy_n_used, post.chunks.len());
    let can_apply = new_state.iter().map(|x| *x == 0 || *x == 1).collect();
    
    let response = DiffStateResponse {
        id: post.id,
        state: applied_state,
        can_apply,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
