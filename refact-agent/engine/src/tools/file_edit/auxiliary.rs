use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::call_validation::DiffChunk;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
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

pub async fn write_file(gcx: Arc<ARwLock<GlobalContext>>, path: &PathBuf, file_text: &String, dry: bool) -> Result<(String, String), String> {
    let parent = path.parent().ok_or(format!(
        "Failed to Add: {:?}. Path is invalid.\nReason: path must have had a parent directory",
        path
    ))?;
    
    if !parent.exists() {
        if !dry {
            fs::create_dir_all(&parent).map_err(|e| {
                let err = format!("Failed to Add: {:?}; Its parent dir {:?} did not exist and attempt to create it failed.\nERROR: {}", path, parent, e);
                warn!("{err}");
                err
            })?;
        }
    }
    
    let before_text = if path.exists() {
        get_file_text_from_memory_or_disk(gcx.clone(), path).await?
    } else {
        "".to_string()
    };
    
    if !dry {
        fs::write(&path, file_text).map_err(|e| {
            let err = format!("Failed to write file: {:?}\nERROR: {}", path, e);
            warn!("{err}");
            err
        })?;
    }
    
    Ok((before_text, file_text.to_string()))
}

pub async fn str_replace(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    old_str: &String,
    new_str: &String,
    replace_multiple: bool,
    dry: bool,
) -> Result<(String, String), String> {
    let file_content = get_file_text_from_memory_or_disk(gcx.clone(), path).await?;

    let has_crlf = file_content.contains("\r\n");

    let normalized_content = normalize_line_endings(&file_content);
    let normalized_old_str = normalize_line_endings(old_str);

    let occurrences = normalized_content.matches(&normalized_old_str).count();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, `old_str` did not appear verbatim in {:?}. Check the file content using `cat()`",
            path
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
            "No replacement was performed. Multiple occurrences of `old_str` in lines {:?}. Please ensure it is unique or set `replace_multiple` to true.",
            lines
        ));
    }

    let normalized_new_str = normalize_line_endings(new_str);
    let new_content = normalized_content.replace(&normalized_old_str, &normalized_new_str);
    let new_file_content = restore_line_endings(&new_content, has_crlf);
    write_file(gcx.clone(), path, &new_file_content, dry).await?;
    Ok((file_content, new_file_content))
}

#[derive(Debug, Clone)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

pub fn parse_line_ranges(ranges_str: &str, total_lines: usize) -> Result<Vec<LineRange>, String> {
    let mut ranges = Vec::new();

    for part in ranges_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let range = if part.contains(':') {
            let parts: Vec<&str> = part.splitn(2, ':').collect();
            let start_str = parts[0].trim();
            let end_str = parts[1].trim();

            let start = if start_str.is_empty() {
                1
            } else {
                start_str.parse::<usize>().map_err(|_| {
                    format!("Invalid start line number '{}' in range '{}'", start_str, part)
                })?
            };

            let end = if end_str.is_empty() {
                total_lines
            } else {
                end_str.parse::<usize>().map_err(|_| {
                    format!("Invalid end line number '{}' in range '{}'", end_str, part)
                })?
            };

            LineRange { start, end }
        } else {
            let line = part.parse::<usize>().map_err(|_| {
                format!("Invalid line number '{}'", part)
            })?;
            LineRange { start: line, end: line }
        };

        if range.start == 0 {
            return Err("Line numbers are 1-based. Start line must be at least 1.".to_string());
        }
        if range.end < range.start {
            return Err(format!(
                "Invalid range '{}': end line ({}) must be >= start line ({}).",
                part, range.end, range.start
            ));
        }
        if range.start > total_lines {
            return Err(format!(
                "Start line {} is beyond end of file ({} lines).",
                range.start, total_lines
            ));
        }

        ranges.push(range);
    }

    if ranges.is_empty() {
        return Err("No valid ranges provided.".to_string());
    }

    ranges.sort_by(|a, b| b.start.cmp(&a.start));

    for i in 0..ranges.len() - 1 {
        let current = &ranges[i];
        let next = &ranges[i + 1];
        if next.end >= current.start {
            return Err(format!(
                "Overlapping ranges detected: {}:{} and {}:{}",
                next.start, next.end, current.start, current.end
            ));
        }
    }

    Ok(ranges)
}

pub async fn str_replace_lines(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    new_content: &String,
    ranges_str: &str,
    dry: bool,
) -> Result<(String, String), String> {
    let file_content = get_file_text_from_memory_or_disk(gcx.clone(), path).await?;
    let has_crlf = file_content.contains("\r\n");

    let normalized_content = normalize_line_endings(&file_content);
    let mut lines: Vec<String> = normalized_content.lines().map(|s| s.to_string()).collect();
    let total_lines = lines.len();

    let ranges = parse_line_ranges(ranges_str, total_lines)?;
    let normalized_new_content = normalize_line_endings(new_content);

    if ranges.len() == 1 {
        let range = &ranges[0];
        let effective_end = range.end.min(total_lines);
        let start_idx = range.start - 1;
        let end_idx = effective_end;
        let new_lines: Vec<String> = normalized_new_content.lines().map(|s| s.to_string()).collect();
        lines.splice(start_idx..end_idx, new_lines);
    } else {
        let content_parts: Vec<&str> = normalized_new_content.split("---RANGE_SEPARATOR---").collect();

        if content_parts.len() != ranges.len() {
            return Err(format!(
                "Content has {} parts (separated by ---RANGE_SEPARATOR---) but {} ranges were specified. \
                 For multiple ranges, separate content for each range with '---RANGE_SEPARATOR---'.",
                content_parts.len(), ranges.len()
            ));
        }

        for (i, range) in ranges.iter().enumerate() {
            let effective_end = range.end.min(lines.len());
            let start_idx = range.start - 1;
            let end_idx = effective_end;
            let content_idx = ranges.len() - 1 - i;
            let part_content = content_parts[content_idx].trim();
            let new_lines: Vec<String> = part_content.lines().map(|s| s.to_string()).collect();
            lines.splice(start_idx..end_idx, new_lines);
        }
    }

    let new_content_joined = lines.join("\n");
    let new_file_content = if normalized_content.ends_with('\n') {
        restore_line_endings(&format!("{}\n", new_content_joined), has_crlf)
    } else {
        restore_line_endings(&new_content_joined, has_crlf)
    };

    write_file(gcx.clone(), path, &new_file_content, dry).await?;
    Ok((file_content, new_file_content))
}

pub async fn str_replace_regex(
    gcx: Arc<ARwLock<GlobalContext>>,
    path: &PathBuf,
    pattern: &Regex,
    replacement: &String,
    multiple: bool,
    dry: bool
) -> Result<(String, String), String> {
    let file_content = get_file_text_from_memory_or_disk(gcx.clone(), path).await?;
    let has_crlf = file_content.contains("\r\n");

    let normalized_content = normalize_line_endings(&file_content);
    let matches: Vec<Match> = pattern.find_iter(&normalized_content).collect();
    let occurrences = matches.len();
    if occurrences == 0 {
        return Err(format!(
            "No replacement was performed, `pattern` did not appear verbatim in {:?}. Check the file content using `cat()`",
            path
        ));
    }
    if !multiple && occurrences > 1 {
        return Err(
            "No replacement was performed. Multiple occurrences of `pattern`. Please ensure the `pattern` is unique or set `multiple` to true.".to_string()
        );
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
    write_file(gcx.clone(), path, &new_file_content, dry).await?;
    Ok((file_content, new_file_content))
}