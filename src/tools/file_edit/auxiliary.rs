use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;
use regex::{Match, Regex};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;

pub fn convert_edit_to_diffchunks(
    path: PathBuf,
    before: &String,
    after: &String,
) -> Result<Vec<DiffChunk>, String> {
    let diffs = diff::lines(before, after);
    let mut line_num = 0;
    let mut current_chunk_lines_remove = Vec::new();
    let mut current_chunk_lines_add = Vec::new();
    let mut current_chunk_line_nums = Vec::new();
    let mut current_chunk_is_plus = Vec::new();
    let mut diff_chunks = Vec::new();

    let flush_changes = |lines_remove: &Vec<String>, 
                        lines_add: &Vec<String>, 
                        line_nums: &Vec<usize>,
                        is_plus: &Vec<bool>| -> Option<DiffChunk> {
        if lines_remove.is_empty() && lines_add.is_empty() {
            return None;
        }

        let lines_remove = lines_remove.join("");
        let lines_add = lines_add.join("");
        
        let line1 = line_nums.iter()
            .min()
            .map(|&x| x + 1)
            .unwrap_or(1);
            
        let line2 = line_nums.iter()
            .zip(is_plus.iter())
            .map(|(&num, &is_plus)| {
                if is_plus {
                    num + 1
                } else {
                    num + 2
                }
            })
            .max()
            .unwrap_or(1);

        Some(DiffChunk {
            file_name: path.to_string_lossy().to_string(),
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1,
            line2,
            lines_remove,
            lines_add,
            ..Default::default()
        })
    };

    for diff in diffs {
        match diff {
            diff::Result::Left(l) => {
                current_chunk_lines_remove.push(format!("{}\n", l));
                current_chunk_line_nums.push(line_num);
                current_chunk_is_plus.push(false);
                line_num += 1;
            }
            diff::Result::Right(r) => {
                current_chunk_lines_add.push(format!("{}\n", r));
                current_chunk_line_nums.push(line_num);
                current_chunk_is_plus.push(true);
            }
            diff::Result::Both(_, _) => {
                if let Some(chunk) = flush_changes(
                    &current_chunk_lines_remove,
                    &current_chunk_lines_add,
                    &current_chunk_line_nums,
                    &current_chunk_is_plus,
                ) {
                    diff_chunks.push(chunk);
                }
                current_chunk_lines_remove.clear();
                current_chunk_lines_add.clear();
                current_chunk_line_nums.clear();
                current_chunk_is_plus.clear();
                line_num += 1;
            }
        }
    }

    if let Some(chunk) = flush_changes(
        &current_chunk_lines_remove,
        &current_chunk_lines_add,
        &current_chunk_line_nums,
        &current_chunk_is_plus,
    ) {
        diff_chunks.push(chunk);
    }

    Ok(diff_chunks)
}

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
    old_str: &String,
    new_str: &String,
    replace_multiple: bool,
) -> Result<(String, String), String> {
    let file_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {:?}\nERROR: {}", path, e))?;

    let has_crlf = file_content.contains("\r\n");

    let normalized_content = normalize_line_endings(&file_content);
    let normalized_old_str = normalize_line_endings(old_str);

    let occurrences = normalized_content.matches(&normalized_old_str).count();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, old_str \n```\n{}\n```\ndid not appear verbatim in {:?}. Consider checking the file content using `cat()`",
            old_str, path
        ));
    }
    if !replace_multiple && occurrences > 1 {
        let lines: Vec<usize> = normalized_content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(&normalized_old_str))
            .map(|(idx, _)| idx + 1)
            .collect();
        return Err(format!(
            "No replacement was performed. Multiple occurrences of old_str `{}` in lines {:?}. Please ensure it is unique or set `replace_multiple` to true.",
            old_str, lines
        ));
    }

    let normalized_new_str = normalize_line_endings(new_str);
    let new_content = normalized_content.replace(&normalized_old_str, &normalized_new_str);

    let new_file_content = restore_line_endings(&new_content, has_crlf);
    write_file(path, &new_file_content)?;
    Ok((file_content, new_file_content))
}

pub fn str_replace_regex(
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
        pattern
            .replace_all(&normalized_content, replacement)
            .to_string()
    } else {
        pattern
            .replace(&normalized_content, replacement)
            .to_string()
    };
    let new_file_content = restore_line_endings(&new_content, has_crlf);
    write_file(path, &new_file_content)?;
    Ok((file_content, new_file_content))
}