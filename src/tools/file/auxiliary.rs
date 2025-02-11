use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;
use itertools::Itertools;
use regex::{Match, Regex};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LineType {
    Plus,
    Minus,
    Space,
}

impl fmt::Display for LineType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match self {
            LineType::Plus => "+",
            LineType::Minus => "-",
            LineType::Space => " ",
        };
        write!(f, "{}", printable)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct DiffLine {
    pub line: String,
    pub line_type: LineType,
    pub file_line_num_idx: Option<usize>,
    pub correct_spaces_offset: Option<i64>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct DiffBlock {
    pub file_name_before: PathBuf,
    pub file_name_after: PathBuf,
    pub action: String,
    pub diff_lines: Vec<DiffLine>,
    pub hunk_idx: usize,
    pub file_lines: Arc<Vec<String>>,
}

pub fn diff_blocks_to_diff_chunks(diff_blocks: &Vec<DiffBlock>) -> Vec<DiffChunk> {
    diff_blocks
        .iter()
        .filter_map(|block| {
            let useful_block_lines = block
                .diff_lines
                .iter()
                .filter(|x| x.line_type != LineType::Space)
                .collect::<Vec<_>>();
            let (filename, filename_rename) = if block.action == "add" {
                (block.file_name_after.to_string_lossy().to_string(), None)
            } else if block.action == "remove" {
                (block.file_name_before.to_string_lossy().to_string(), None)
            } else if block.action == "rename" {
                (block.file_name_before.to_string_lossy().to_string(),
                 Some(block.file_name_after.to_string_lossy().to_string()))
            } else {  // edit
                assert_eq!(block.file_name_before, block.file_name_after);
                (block.file_name_before.to_string_lossy().to_string(), None)
            };
            let lines_remove = useful_block_lines
                .iter()
                .filter(|x| x.line_type == LineType::Minus)
                .map(|x| format!("{}\n", x.line.clone()))
                .join("");
            let lines_add = useful_block_lines
                .iter()
                .filter(|x| x.line_type == LineType::Plus)
                .map(|x| format!("{}\n", x.line.clone()))
                .join("");
            Some(DiffChunk {
                file_name: filename,
                file_name_rename: filename_rename,
                file_action: block.action.clone(),
                line1: useful_block_lines
                    .iter()
                    .map(|x| x.file_line_num_idx
                        .clone()
                        .expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 1)
                    .min()
                    .unwrap_or(1),
                line2: useful_block_lines
                    .iter()
                    .map(|x| {
                        if x.line_type == LineType::Plus {
                            x.file_line_num_idx.clone()
                                .expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 1
                        } else {
                            x.file_line_num_idx
                                .clone()
                                .expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 2
                        }
                    })
                    .max()
                    .unwrap_or(1),
                lines_remove,
                lines_add,
                ..Default::default()
            })
        })
        .collect()
}

pub fn convert_edit_to_diffchunks(
    path: PathBuf,
    before: &String,
    after: &String,
) -> Result<Vec<DiffChunk>, String> {
    pub fn chunks_from_diffs(
        file_path: PathBuf,
        diffs: Vec<diff::Result<&str>>,
    ) -> Result<Vec<DiffChunk>, String> {
        let mut line_num: usize = 0;
        let mut blocks = vec![];
        let mut diff_lines = vec![];
        for diff in diffs {
            match diff {
                diff::Result::Left(l) => {
                    diff_lines.push(DiffLine {
                        line: l.to_string(),
                        line_type: LineType::Minus,
                        file_line_num_idx: Some(line_num),
                        correct_spaces_offset: Some(0),
                    });
                    line_num += 1;
                }
                diff::Result::Right(r) => {
                    diff_lines.push(DiffLine {
                        line: r.to_string(),
                        line_type: LineType::Plus,
                        file_line_num_idx: Some(line_num),
                        correct_spaces_offset: Some(0),
                    });
                }
                diff::Result::Both(_, _) => {
                    line_num += 1;
                    if !diff_lines.is_empty() {
                        blocks.push(DiffBlock {
                            file_name_before: file_path.clone(),
                            file_name_after: file_path.clone(),
                            action: "edit".to_string(),
                            file_lines: Arc::new(vec![]),
                            hunk_idx: 0,
                            diff_lines: diff_lines.clone(),
                        });
                        diff_lines.clear();
                    }
                }
            }
        }
        if !diff_lines.is_empty() {
            blocks.push(DiffBlock {
                file_name_before: file_path.clone(),
                file_name_after: file_path.clone(),
                action: "edit".to_string(),
                file_lines: Arc::new(vec![]),
                hunk_idx: 0,
                diff_lines: diff_lines.clone(),
            });
            diff_lines.clear();
        }

        Ok(diff_blocks_to_diff_chunks(&blocks))
    }

    let diffs = diff::lines(&before, &after);
    chunks_from_diffs(path.clone(), diffs)
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
