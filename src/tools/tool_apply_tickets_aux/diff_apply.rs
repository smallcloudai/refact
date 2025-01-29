use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::call_validation::DiffChunk;
use crate::diffs::{correct_and_validate_chunks, read_files_n_apply_diff_chunks, unwrap_diff_apply_outputs, ApplyDiffResult, ApplyDiffUnwrapped};
use crate::files_in_workspace::{read_file_from_disk, Document};
use crate::global_context::GlobalContext;
use crate::privacy::load_privacy_if_needed;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use itertools::multizip;

const MAX_FUZZY_N: usize = 10;

async fn write_results_on_disk(
    gcx: Arc<ARwLock<GlobalContext>>,
    results: Vec<ApplyDiffResult>,
) -> Result<Vec<Document>, String> {
    async fn write_to_file(path: &String, text: &str) -> Result<(), String> {
        let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(path).await
            .map_err(|e| format!("Failed to open file {}\nERROR: {}", path, e))?;
        file.write_all(text.as_bytes()).await
            .map_err(|e| format!("Failed to write into file {}\nERROR: {}", path, e))?;
        Ok(())
    }
    fn apply_add_action(path_str: &String, file_text: &String) -> Result<(), String> {
        let path = PathBuf::from(path_str);
        let parent = path.parent().ok_or(format!("Failed to Add: {}. Path is invalid.\nReason: path must have had a parent directory", path_str))?;
        if !parent.exists() {
            fs::create_dir_all(&parent).map_err(|e| {
                let err = format!("Failed to Add: {:?}; Its parent dir {:?} did not exist and attempt to create it failed.\nERROR: {}", path, parent, e);
                warn!("{err}");
                err
            })?;
        }
        fs::write(&path, file_text).map_err(|e| {
            let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
            warn!("{err}");
            err
        })
    }
    fn apply_remove_action(path_str: &String) -> Result<(), String> {
        let path = PathBuf::from(path_str);
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| {
                let err = format!("Failed to Remove file: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        } else if path.is_dir() {
            fs::remove_dir(&path).map_err(|e| {
                let err = format!("Failed to Remove dir: {:?}\nERROR: {}", path, e);
                warn!("{err}");
                err
            })
        } else {
            Err(format!("Failed to Remove: path '{}' does not exist", path_str))
        }
    }
    fn apply_rename_action(rename_from: &String, rename_into: &String) -> Result<(), String> {
        if PathBuf::from(rename_into).exists() {
            let err = format!("Failed to Rename: path '{}' (rename into) already exists", rename_into);
            warn!("{err}");
            return Err(err);
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
        } else if r.file_name_delete.is_some() && r.file_name_add.is_some() {
            let rename_from = &r.file_name_delete.unwrap();
            let rename_into = &r.file_name_add.unwrap();
            apply_rename_action(rename_from, rename_into)?;
            if PathBuf::from(rename_into).is_file() {
                let mut doc = Document::new(&PathBuf::from(rename_into));
                let text = read_file_from_disk(load_privacy_if_needed(gcx.clone()).await, &doc.doc_path).await?.to_string();
                doc.update_text(&text);
                docs2index.push(doc);
            }
        } else if r.file_name_add.is_some() && r.file_text.is_some() {
            let path_add = &r.file_name_add.unwrap();
            apply_add_action(path_add, &r.file_text.clone().unwrap())?;
            if PathBuf::from(path_add).is_file() {
                let mut doc = Document::new(&PathBuf::from(path_add));
                doc.update_text(&r.file_text.unwrap());
                docs2index.push(doc);
            }
        } else if r.file_name_delete.is_some() {
            apply_remove_action(&r.file_name_delete.unwrap())?;
        }
    }
    Ok(docs2index)
}

async fn set_chunks_detail_and_sync_documents_ast_vecdb(
    gcx: Arc<ARwLock<GlobalContext>>,
    new_documents: Vec<Document>,
    apply_outputs: Vec<ApplyDiffUnwrapped>,
    chunks: &mut Vec<DiffChunk>,
) -> Result<(), String> {
    let ast_service_mb = gcx.read().await.ast_service.clone();

    for (doc, apply_output, chunk) in multizip((new_documents.iter(), apply_outputs.iter(), chunks.iter_mut())) {
        if apply_output.applied {
            chunk.application_details = "Chunk applied successfully".to_string();
            if let Some(ast_service) = &ast_service_mb {
                ast_indexer_enqueue_files(
                    ast_service.clone(),
                    &vec![doc.doc_path.to_string_lossy().to_string()],
                    true,
                ).await;
            }
        } else {
            if let Some(error) = &apply_output.detail {
                if !error.is_empty() {
                    chunk.application_details = error.clone();
                } else {
                    chunk.application_details = "Couldn't apply the chunk due to an unknown error".to_string();
                }
            } else {
                chunk.application_details = "Couldn't apply the chunk due to an unknown error".to_string();
            }
        }
    }
    if let Some(ast_service) = &ast_service_mb {
        ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;
    }
    Ok(())
}

pub async fn diff_apply(
    gcx: Arc<ARwLock<GlobalContext>>,
    chunks: &mut Vec<DiffChunk>,
) -> Result<(), String> {
    correct_and_validate_chunks(gcx.clone(), chunks).await?;
    let (results, outputs) = read_files_n_apply_diff_chunks(
        gcx.clone(),
        &chunks,
        &chunks.iter().map(|_| false).collect(),
        &chunks.iter().map(|_| true).collect(),
        MAX_FUZZY_N,
    ).await;
    let new_documents = write_results_on_disk(
        gcx.clone(), results.clone(),
    ).await?;
    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());
    set_chunks_detail_and_sync_documents_ast_vecdb(gcx.clone(), new_documents, outputs_unwrapped, chunks).await
}
