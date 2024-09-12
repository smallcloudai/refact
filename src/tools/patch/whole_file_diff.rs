use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::patch::chat_interaction::read_file;
use crate::tools::patch::snippets::CodeSnippet;
use crate::tools::patch::unified_diff_format::{diff_blocks_to_diff_chunks, DiffBlock, DiffLine, LineType};
use crate::call_validation::DiffChunk;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;


pub async fn full_rewrite_diff(
    ccx: Arc<AMutex<AtCommandsContext>>,
    snippet: &CodeSnippet,
) -> Result<Vec<DiffChunk>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let context_file = read_file(gcx.clone(), snippet.filename_before.clone()).await
        .map_err(|e|format!("cannot read file to modify: {}.\nError: {e}", snippet.filename_before))?;
    let file_name = PathBuf::from(&context_file.file_name);

    let diffs = diff::lines(&context_file.file_content, &snippet.code);
    let mut line_num: usize = 0;
    let mut blocks: Vec<DiffBlock> = vec![];
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
                        file_name_before: file_name.clone(),
                        file_name_after: file_name.clone(),
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
            file_name_before: file_name.clone(),
            file_name_after: file_name.clone(),
            action: "edit".to_string(),
            file_lines: Arc::new(vec![]),
            hunk_idx: 0,
            diff_lines: diff_lines.clone(),
        });
        diff_lines.clear();
    }

    Ok(diff_blocks_to_diff_chunks(&blocks))
}

pub fn new_file_diff(
    snippet: &CodeSnippet,
) -> Vec<DiffChunk> {
    vec![
        DiffChunk {
            file_name: snippet.filename_before.clone(),
            file_name_rename: None,
            file_action: "add".to_string(),
            line1: 1,
            line2: 1,
            lines_remove: "".to_string(),
            lines_add: snippet.code.clone(),
            ..Default::default()
        }
    ]
}