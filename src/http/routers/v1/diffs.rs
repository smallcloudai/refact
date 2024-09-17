use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use tracing::warn;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::DiffChunk;
use crate::custom_error::ScratchError;
use crate::diffs::{unwrap_diff_apply_outputs, correct_and_validate_chunks, ApplyDiffResult, read_files_n_apply_diff_chunks, ApplyDiffUnwrapped};
use crate::files_in_workspace::{Document, read_file_from_disk};
use crate::global_context::GlobalContext;
use crate::privacy::load_privacy_if_needed;
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

async fn write_results_on_disk(
    gcx: Arc<ARwLock<GlobalContext>>,
    results: Vec<ApplyDiffResult>
) -> Result<Vec<Document>, String> {
    async fn write_to_file(path: &String, text: &str) -> Result<(), String> {
        let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(path).await
            .map_err(|e| format!("Failed to open file {}\nERROR: {}", path, e))?;
        file.write_all(text.as_bytes()).await
            .map_err(|e|format!("Failed to write into file {}\nERROR: {}", path, e))?;
        Ok(())
    }

    fn apply_add_action(path_str: &String, file_text: &String) -> Result<(), String> {
        let path = PathBuf::from(path_str);
        let parent = path.parent().ok_or(format!("Failed to Add: {}. Path is invalid.\nReason: path must have had a parent directory", path_str))?;
        if !parent.exists() {
            fs::create_dir_all(&parent).map_err(|e| {
                let err= format!("Failed to Add: {:?}; Its parent dir {:?} did not exist and attempt to create it failed.\nERROR: {}", path, parent, e);
                warn!("{err}");
                err
            })?;
        }

        if path.extension().is_some() {
            fs::write(&path, file_text).map_err(|e| {
                let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        } else {
            fs::create_dir_all(&path).map_err(|e| {
                let err = format!("Failed to create dir: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        }
    }

    fn apply_remove_action(path_str: &String) -> Result<(), String> {
        let path = PathBuf::from(path_str);
        return if path.is_file() {
            fs::remove_file(&path).map_err(|e| {
                let err = format!("Failed to Remove file: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        }
        else if path.is_dir() {
            fs::remove_dir(&path).map_err(|e| {
                let err = format!("Failed to Remove dir: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        } else {
            return Err(format!("Failed to Remove: path '{}' does not exist", path_str))
        }
    }

    fn apply_rename_action(rename_from: &String, rename_into: &String) -> Result<(), String> {
        if PathBuf::from(rename_into).exists() {
            let err = format!("Failed to Rename: path '{}' (rename into) already exists", rename_into);
            warn!("{err}");
            return Err(err)
        }
        if PathBuf::from(rename_from).exists() {
            fs::rename(rename_from, rename_into).map_err(|e| {
                let err = format!("Failed to Rename: path '{}' (rename from) does not exist.\nERROR: {}", rename_from, e);
                warn!("{err}");
                err
            })
        } else {
            let err = format!("Failed to Rename: path '{}' (rename from) does not exist", rename_from);
            Err(err)
        }
    }

    let mut docs2index = vec![];

    for r in results {
        if r.file_name_edit.is_some() && r.file_text.is_some() {
            write_to_file(&r.file_name_edit.clone().unwrap(), &r.file_text.clone().unwrap()).await?;
            let mut doc = Document::new(&PathBuf::from(&r.file_name_edit.unwrap()));
            doc.update_text(&r.file_text.unwrap());
            docs2index.push(doc);
        }
        else if r.file_name_delete.is_some() && r.file_name_add.is_some() {
            let rename_from = &r.file_name_delete.unwrap();
            let rename_into = &r.file_name_add.unwrap();
            apply_rename_action(rename_from, rename_into)?;
            if PathBuf::from(rename_into).is_file() {
                let mut doc = Document::new(&PathBuf::from(rename_into));
                let text = read_file_from_disk(load_privacy_if_needed(gcx.clone()).await, &doc.doc_path).await?.to_string();
                doc.update_text(&text);
                docs2index.push(doc);
            }
        }
        else if r.file_name_add.is_some() && r.file_text.is_some() {
            let path_add = &r.file_name_add.unwrap();
            apply_add_action(path_add, &r.file_text.clone().unwrap())?;
            if PathBuf::from(path_add).is_file() {
                let mut doc = Document::new(&PathBuf::from(path_add));
                doc.update_text(&r.file_text.unwrap());
                docs2index.push(doc);
            }
        }
        else if r.file_name_delete.is_some() {
            apply_remove_action(&r.file_name_delete.unwrap())?;
        }
    }
    Ok(docs2index)
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

#[allow(dead_code)]
async fn _sync_documents_ast_vecdb(gcx: Arc<ARwLock<GlobalContext>>, docs: Vec<Document>) -> Result<(), ScratchError>
{
    // XXX: blocking should happen before any tool calls, not after applying diffs

    let ast_service_opt = gcx.read().await.ast_service.clone();
    if let Some(ast_service) = ast_service_opt {
        crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 1_000, true).await;
    }

    let vecdb_enqueued = if let Some(vservice) = {
        let gcx_locked = gcx.write().await;
        let vec_db_guard = gcx_locked.vec_db.lock().await;
        vec_db_guard.as_ref().map(|v| v.vectorizer_service.clone())
    } {
        vectorizer_enqueue_files(vservice, &docs, true).await;
        true
    } else {
        false
    };

    if vecdb_enqueued {
        let vecdb = gcx.write().await.vec_db.clone();
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
    correct_and_validate_chunks(global_context.clone(), &mut post.chunks).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default()
    };
    let desired_state = post.apply.clone();

    let (results, outputs) = read_files_n_apply_diff_chunks(load_privacy_if_needed(global_context.clone()).await, &post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);

    // XXX: blocking should happen before any tool calls, not after applying diffs
    // let docs2index = write_results_on_disk(results.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    // sync_documents_ast_vecdb(global_context.clone(), docs2index).await?;
    
    let new_documents = write_results_on_disk(global_context.clone(), results.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;
    
    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, post.chunks);

    {
        let mut gcx_lock = global_context.write().await;
        gcx_lock.documents_state.diffs_applied_state.insert(post.id, outputs_unwrapped.iter().map(|x|x.applied == true).collect::<Vec<_>>());

        let cache_dirty = gcx_lock.documents_state.cache_dirty.clone();
        *cache_dirty.lock().await = true;
        
        for doc in new_documents {
            gcx_lock.documents_state.memory_document_map.insert(doc.doc_path.clone(), Arc::new(ARwLock::new(doc)));
        }
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&outputs_unwrapped).unwrap()))
        .unwrap())
}

#[derive(Serialize)]
struct DiffPreviewResponse {
    state: Vec<ApplyDiffUnwrapped>,
    results: Vec<ApplyDiffResult>,
}

pub async fn handle_v1_diff_preview(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    validate_post(&post)?;
    correct_and_validate_chunks(global_context.clone(), &mut post.chunks).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default()
    };
    let desired_state = post.apply.clone();
    let (results, outputs) = read_files_n_apply_diff_chunks(load_privacy_if_needed(global_context.clone()).await, &post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);

    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, post.chunks);

    let resp = DiffPreviewResponse {
        state: outputs_unwrapped,
        results,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&resp).unwrap()))
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

// WARNING! DEPRECATED
pub async fn handle_v1_diff_state(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffStatePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    correct_and_validate_chunks(global_context.clone(), &mut post.chunks).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).cloned().unwrap_or_default()
    };
    let desired_state = vec![true; post.chunks.len()];

    let (_, outputs) = read_files_n_apply_diff_chunks(load_privacy_if_needed(global_context.clone()).await, &post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);
    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, post.chunks);

    let can_apply = outputs_unwrapped.iter().map(|x|x.applied).collect::<Vec<_>>();

    let response = DiffStateResponse {
        id: post.id,
        state: applied_state,
        can_apply,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("WARNING", "This endpoint is deprecated. Use /v1/diff-preview instead")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
