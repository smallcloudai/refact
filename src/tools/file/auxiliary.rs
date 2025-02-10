use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::global_context::GlobalContext;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use regex::{Match, Regex};
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::call_validation::DiffChunk;
use crate::tools::tool_apply_edit_aux::diff_structs::chunks_from_diffs;

pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

pub fn restore_line_endings(content: &str, original_had_crlf: bool) -> String {
    if original_had_crlf {
        content.replace("\n", "\r\n")
    } else {
        content.to_string()
    }
}

pub async fn await_ast_indexing(gcx: Arc<ARwLock<GlobalContext>>) -> Result<(), String> {
    let ast_service_mb = gcx.read().await.ast_service.clone();
    if let Some(ast_service) = &ast_service_mb {
        ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;
    }
    Ok(())
}

pub async fn sync_documents_ast(
    gcx: Arc<ARwLock<GlobalContext>>,
    doc: &PathBuf,
) -> Result<(), String> {
    let ast_service_mb = gcx.read().await.ast_service.clone();
    if let Some(ast_service) = &ast_service_mb {
        ast_indexer_enqueue_files(
            ast_service.clone(),
            &vec![doc.to_string_lossy().to_string()],
            true,
        )
        .await;
    }
    Ok(())
}

pub fn write_file(path: &PathBuf, file_text: &String) -> Result<(String, String), String> {
    if !path.exists() {
        let parent = path.parent().ok_or(format!(
            "Failed to Add: {:?}. Path is invalid.\nReason: path must have had a parent directory",
            path
        ))?;
        if !parent.exists() {
            fs::create_dir_all(&parent).map_err(|e| {
                let err = format!("Failed to Add: {:?}; Its parent dir {:?} did not exist and attempt to create it failed.\nERROR: {}", path, parent, e);
                warn!("{err}");
                err
            })?;
        }
    }
    let before_text = if path.exists() {
        fs::read_to_string(&path).map_err(|x| x.to_string())?
    } else {
        "".to_string()
    };
    fs::write(&path, file_text).map_err(|e| {
        let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
        warn!("{err}");
        err
    })?;
    Ok((before_text, file_text.to_string()))
}

pub fn str_replace(
    path: &PathBuf,
    pattern: &Regex,
    replacement: &String,
    multiple: bool,
) -> Result<(String, String), String> {
    let file_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {:?}\nERROR: {}", path, e))?;
    let has_crlf = file_content.contains("\r\n");

    let normalized_content = normalize_line_endings(&file_content);
    let matches: Vec<Match> = pattern.find_iter(&normalized_content).collect();
    let occurrences = matches.len();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, `pattern` \n```\n{}\n```\ndid not appear verbatim in {:?}. Consider checking the file content using `cat()`",
            pattern.to_string(), path
        ));
    }
    if !multiple && occurrences > 1 {
        return Err(format!(
            "No replacement was performed. Multiple occurrences of `pattern` `{}`. Please ensure the `pattern` is unique or set `multiple` to true.",
            pattern.to_string()
        ));
    }
    let new_content = if multiple && occurrences > 1 {
        pattern.replace_all(&normalized_content, replacement).to_string()
    } else {
        pattern.replace(&normalized_content, replacement).to_string()
    };
    let new_file_content = restore_line_endings(&new_content, has_crlf);
    write_file(path, &new_file_content)?;
    Ok((file_content, new_file_content))
}


pub fn convert_edit_to_diffchunks(path: PathBuf, before: &String, after: &String) -> Result<Vec<DiffChunk>, String> {
    let diffs = diff::lines(&before, &after);
    chunks_from_diffs(path.clone(), diffs)
}
