use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hashbrown::HashMap;
use hyper::Body;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::call_validation::DiffChunk;
use crate::custom_error::ScratchError;
use crate::diffs::{read_files_n_apply_diff_chunks_edit, fuzzy_results_into_state_vector, correct_and_validate_chunks, can_apply_diff_chunks_other, apply_diff_chunks_other_to_files};
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::vdb_highlev::memories_block_until_vectorized;
use crate::vecdb::vdb_thread::vectorizer_enqueue_files;


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

fn validate_post(post: &DiffPost) -> Result<(), ScratchError> {
    if post.chunks.is_empty() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` shouldn't be empty".to_string()));
    }
    if post.chunks.len() != post.apply.len() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` and `apply` arrays are not of the same length".to_string()));
    }
    Ok(())
}

async fn sync_documents_ast_vecdb(global_context: Arc<ARwLock<GlobalContext>>, docs: Vec<Document>) -> Result<(), ScratchError> {
    if let Some(ast) = global_context.write().await.ast_module.clone() {
        let mut ast_write = ast.write().await;
        for doc in docs.iter() {
            ast_write.ast_add_file_no_queue(doc, true).await.map_err(|e| {
                let e_text = format!("Failed to sync doc {:?} with AST. Error:\n{}", doc.path, e);
                warn!(e_text);
                ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e_text)
            })?;
        }
    }
    
    let vecdb_enqueued = if let Some(vservice) = {
        let cx = global_context.write().await;
        let vec_db_guard = cx.vec_db.lock().await;
        vec_db_guard.as_ref().map(|v| v.vectorizer_service.clone())
    } {
        vectorizer_enqueue_files(vservice, &docs, true).await;
        true
    } else {
        false
    };
    
    if vecdb_enqueued {
        let vecdb = global_context.write().await.vec_db.clone();
        memories_block_until_vectorized(vecdb).await.map_err(|e| 
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
        )?;
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
    correct_and_validate_chunks(&mut post.chunks, global_context.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    // undo all chunks that are already applied to file, then apply chunks marked in post.apply
    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default()
    };
    let desired_state = post.apply.clone();
    let (texts_after_patch_edit, fuzzy_n_used) = read_files_n_apply_diff_chunks_edit(&post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);
    
    // println!("applied_state: {:?}", applied_state);
    let can_apply_other_raw = can_apply_diff_chunks_other(&post.chunks, &applied_state, &desired_state);
    let can_apply_other = fuzzy_results_into_state_vector(&can_apply_other_raw, post.chunks.len()).iter().map(|x| *x == 0 || *x == 1).collect();
    // println!("can_apply_other: {:?}", can_apply_other);
    let results_other = apply_diff_chunks_other_to_files(&post.chunks, &can_apply_other);

    for (file_name, new_text) in texts_after_patch_edit.iter() {
        write_to_file(file_name, new_text).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    }

    let docs: Vec<Document> = texts_after_patch_edit.iter().map(|(k, v)| {
        let mut doc = Document::new(&PathBuf::from(k));
        doc.update_text(v);
        doc
    }).collect();

    sync_documents_ast_vecdb(global_context.clone(), docs).await?;
    
    let mut fuzzy_results_map = HashMap::new();
    fuzzy_results_map.extend(results_other);
    fuzzy_results_map.extend(fuzzy_n_used);
        
    let new_state = fuzzy_results_into_state_vector(&fuzzy_results_map, post.chunks.len());
    global_context.write().await.documents_state.diffs_applied_state.insert(post.id, new_state.iter().map(|x|x==&1).collect::<Vec<_>>().clone());

    let fuzzy_results: Vec<DiffResponseItem> = fuzzy_results_map.iter().filter(|x|x.1.is_some())
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
        .header("Content-Type", "application/json")
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
    state: Vec<bool>,
    can_apply: Vec<bool>,
}

pub async fn handle_v1_diff_state(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffStatePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    correct_and_validate_chunks(&mut post.chunks, global_context.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).cloned().unwrap_or_default()
    };
    let desired_state = vec![true; post.chunks.len()];

    let (_, fuzzy_n_used) = read_files_n_apply_diff_chunks_edit(&post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);
    let can_apply_other = can_apply_diff_chunks_other(&post.chunks, &applied_state, &desired_state);
    
    let mut can_apply_raw = HashMap::new();
    can_apply_raw.extend(fuzzy_n_used);
    can_apply_raw.extend(can_apply_other);
    
    let new_state = fuzzy_results_into_state_vector(&can_apply_raw, post.chunks.len());
    let can_apply = new_state.iter().map(|x| *x == 0 || *x == 1).collect();

    let response = DiffStateResponse {
        id: post.id,
        state: applied_state,
        can_apply,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
