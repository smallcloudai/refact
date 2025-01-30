use std::path::PathBuf;
use std::sync::Arc;

use crate::call_validation::DiffChunk;
use crate::tools::tool_apply_ticket_aux::diff_structs::{
    diff_blocks_to_diff_chunks, DiffBlock, DiffLine, LineType,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tracing::{error, warn};

use crate::global_context::GlobalContext;
use crate::tools::tool_apply_ticket_aux::fs_utils::read_file;
use crate::tools::tool_apply_ticket_aux::postprocessing_utils::{minimal_common_indent, place_indent};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum SectionType {
    Original,
    Modified,
}

#[derive(Clone, Debug)]
pub struct EditSection {
    hunk: Vec<String>,
    type_: SectionType,
}

fn process_fenced_block(
    lines: &[&str],
    start_line_num: usize,
    is_original: bool,
) -> (usize, EditSection) {
    let mut line_num = start_line_num;
    while line_num < lines.len() {
        if lines[line_num].starts_with("```") {
            break;
        }
        line_num += 1;
    }
    (
        line_num + 1,
        EditSection {
            hunk: lines[start_line_num..line_num]
                .iter()
                .map(|x| x.to_string())
                .collect(),
            type_: if is_original {
                SectionType::Original
            } else {
                SectionType::Modified
            },
        },
    )
}

fn get_edit_sections(content: &str) -> Vec<EditSection> {
    let lines: Vec<&str> = content.lines().collect();
    let mut line_num = 0;
    let mut sections: Vec<EditSection> = vec![];
    while line_num < lines.len() - 1 {
        while line_num < lines.len() - 1 {
            let line = lines[line_num];
            if line.contains("Original Section") {
                let (new_line_num, section) = process_fenced_block(&lines, line_num + 2, true);
                line_num = new_line_num;
                sections.push(section);
                break;
            }
            if line.contains("Modified Section") {
                let (new_line_num, section) = process_fenced_block(&lines, line_num + 2, false);
                line_num = new_line_num;
                sections.push(section);
                break;
            }
            line_num += 1;
        }
    }
    sections
}

fn search_block_line_by_line(
    file_text: &Vec<String>,
    block_to_find: &Vec<String>,
) -> Result<Vec<(usize, usize, Vec<String>)>, String> {
    let mut found: Vec<(usize, usize, Vec<String>)> = vec![];
    let mut block_index = 0;
    let mut current_start = None;
    let mut current_block = vec![];

    for (file_index, file_line) in file_text.iter().enumerate() {
        if file_line.trim_start() == block_to_find[block_index].trim_start() {
            if current_start.is_none() {
                current_start = Some(file_index);
            }
            current_block.push(file_line.clone());
            block_index += 1;

            if block_index == block_to_find.len() {
                break;
            }
        } else {
            if !current_block.is_empty() {
                found.push((
                    current_start.unwrap(),
                    file_index,
                    std::mem::take(&mut current_block),
                ));
                current_start = None;
                current_block.clear();
            }
        }
    }
    if !current_block.is_empty() {
        found.push((
            current_start.unwrap(),
            file_text.len(),
            std::mem::take(&mut current_block),
        ));
    }

    if found.is_empty() {
        Err(format!(
            "Block not found in the file text: {:?}",
            block_to_find
        ))
    } else {
        Ok(found)
    }
}

async fn sections_to_diff_blocks(
    gcx: Arc<ARwLock<GlobalContext>>,
    sections: &Vec<EditSection>,
    filename: &PathBuf,
) -> Result<Vec<DiffBlock>, String> {
    let mut diff_blocks = vec![];
    let file_lines = read_file(gcx.clone(), filename.to_string_lossy().to_string())
        .await
        .map(|x| {
            x.file_content
                .lines()
                .into_iter()
                .map(|x| {
                    if let Some(stripped_row) =
                        x.to_string().replace("\r\n", "\n").strip_suffix("\n")
                    {
                        stripped_row.to_string()
                    } else {
                        x.to_string()
                    }
                })
                .collect::<Vec<_>>()
        })?;
    let mut errors: Vec<String> = vec![];
    for (idx, sections) in sections
        .iter()
        .chunks(2)
        .into_iter()
        .map(|x| x.collect::<Vec<_>>())
        .enumerate()
    {
        let orig_section = sections.get(0).ok_or("No original section found")?;
        let modified_section = sections.get(1).ok_or("No modified section found")?;
        if orig_section.type_ != SectionType::Original
            || modified_section.type_ != SectionType::Modified
        {
            return Err("section types are messed up, try to regenerate the diff".to_string());
        }
        let orig_section_span = orig_section
            .hunk
            .iter()
            .map(|x| x.trim_start().to_string())
            .collect::<Vec<_>>();
        let mut start_offset = None;
        for file_line_idx in 0..=file_lines.len().saturating_sub(orig_section.hunk.len()) {
            let file_lines_span = file_lines
                [file_line_idx..(file_line_idx + orig_section.hunk.len()).min(file_lines.len())]
                .iter()
                .map(|x| x.trim_start().to_string())
                .collect::<Vec<_>>();
            if file_lines_span == orig_section_span {
                start_offset = Some(file_line_idx);
                break;
            }
        }
        if let Some(start_offset) = start_offset {
            let file_section =
                file_lines[start_offset..start_offset + orig_section.hunk.len()].to_vec();
            let (indent_spaces, indent_tabs) =
                minimal_common_indent(&file_section.iter().map(|x| x.as_str()).collect::<Vec<_>>());
            let modified_section_hunk = place_indent(
                &modified_section
                    .hunk
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                indent_spaces,
                indent_tabs,
            );
            diff_blocks.push(DiffBlock {
                file_name_before: filename.clone(),
                file_name_after: filename.clone(),
                action: "edit".to_string(),
                diff_lines: file_lines[start_offset..start_offset + orig_section.hunk.len()]
                    .iter()
                    .enumerate()
                    .map(|(idx, x)| DiffLine {
                        line: x.clone(),
                        line_type: LineType::Minus,
                        file_line_num_idx: Some(start_offset + idx),
                        correct_spaces_offset: None,
                    })
                    .chain(modified_section_hunk.iter().map(|x| DiffLine {
                        line: x.clone(),
                        line_type: LineType::Plus,
                        file_line_num_idx: Some(start_offset),
                        correct_spaces_offset: None,
                    }))
                    .collect::<Vec<_>>(),
                hunk_idx: idx,
                file_lines: Arc::new(vec![]),
            })
        } else {
            match search_block_line_by_line(&file_lines, &orig_section.hunk) {
                Ok(res) => {
                    let mut err = format!(
                        "This section wasn't found in the original file content:\n```\n{}\n```\n",
                        orig_section.hunk.iter().join("\n")
                    );
                    err += "Split it into multiple sections like this:\n";
                    for (_, _, found_block) in res {
                        err += &format!(
                            "### Original Section (to be replaced)\n```\n{}\n```\n",
                            found_block.join("\n")
                        );
                        err += &"### Modified Section (to replace with)\n```\n[Modified code section]\n```\n".to_string();
                    }
                    errors.push(err.clone());
                    error!("{}", err);
                    continue;
                }
                Err(_) => {
                    let err = format!(
                        "This section wasn't found in the original file content:\n```\n{}\n```\n",
                        orig_section.hunk.iter().join("\n")
                    );
                    errors.push(err.clone());
                    error!("{}", err);
                    continue;
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(diff_blocks)
    } else {
        Err(errors.join("\n"))
    }
}

pub fn section_edit_choose_correct_chunk(
    chunks: Vec<Result<Vec<DiffChunk>, String>>,
) -> Result<Vec<DiffChunk>, String> {
    let errors = chunks
        .iter()
        .filter(|res| res.is_err())
        .map(|res| res.clone().unwrap_err())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        warn!("There is a list of errors for some generated diffs");
        for err in errors.iter() {
            warn!("{err}");
        }
    }
    if chunks.iter().all(|res| res.is_err()) {
        let mut err_message = "No valid chunks were generated, reasons are:\n".to_string();
        for err in errors.iter().unique() {
            err_message.push_str(format!("- {err}\n").as_str());
        }
        err_message
            .push_str("Try to call `apply_ticket` one more time to generate a correct diff");
        return Err(err_message);
    }

    let non_error_chunks = chunks
        .iter()
        .filter_map(|res| res.as_ref().ok())
        .cloned()
        .collect::<Vec<_>>();
    warn!("{} diff were parsed successfully", non_error_chunks.len());

    // return the most common chunk
    let mut chunks_freq = HashMap::new();
    for chunk in non_error_chunks.iter() {
        *chunks_freq.entry(chunk).or_insert(0) += 1;
    }
    let max_repeats = chunks_freq
        .iter()
        .max_by_key(|(_, k)| *k)
        .unwrap()
        .1
        .clone();
    let chunks_max_repeats = chunks_freq
        .iter()
        .filter(|(_, v)| **v == max_repeats)
        .map(|x| *x.0)
        .collect::<Vec<_>>();
    Ok((*chunks_max_repeats
        .iter()
        .max()
        .expect("There is no max repeats"))
    .clone())
}
pub struct BlocksOfCodeParser {}

impl BlocksOfCodeParser {
    pub fn prompt() -> String {
        let prompt = r#"**You will be given:
1. An **original file** (the complete, unmodified content).
2. **Modified sections** (portions of the file that have changed).
3. **Hint messages** (optional but important clues about how and where to place changes).

Your task is to:
1. Identify **all** original sections in the file that correspond to the provided modified sections.
2. **Extract** these original sections exactly as they appear (including all indentation, spacing, and formatting).
3. Pair each original section with its corresponding modified version using the **exact output format** provided below.
4. Use hints (if any) to determine the correct placement or grouping of changes.

---

## Steps

1. **Locate Modified Sections**  
   Compare the original file with the modified sections. Identify every instance where the content differs. Some modifications may span entire functions or classes, while others may be smaller. Pay attention to any hints that indicate specific lines or blocks of code.

2. **Output Modifications**  
   Produce your response in the exact format shown below. **Preserve indentation** and other formatting details from the original file to avoid introducing new errors.  
   - If a single original section has multiple, non-contiguous changes, split it into separate pairs of "Original" and "Modified" sections to reflect each distinct change.
   - If a new block of code is added in the modified version (with no direct equivalent in the original), still provide an “Original Section” block indicating where the new code is inserted, and a “Modified Section” block showing the old plus the new code.

---

## Output Format

For **each** modification, use the exact structure shown below. 

```
### Original Section (to be replaced)
```
```
[the exact original section content here]
```
```
### Modified Section (to replace with)
```
```
[the exact modified section content here]
```

---

## Notes

- Sometimes modified sections may contain lines like "Previous content remains the same". Do not consider it as real changes but consider it as hints. Do not copy these lines to the original file.
- **Entire Functions vs. Small Changes:** Whenever possible, replace or show changes for the entire function, method, or logical block rather than multiple small snippets.
- **Multiple Changes in a Single Section:** If multiple modifications occur in different parts of the same original section, create separate "Original/Modified" pairs for each.
- **Preserve Indentation and Formatting:** Do not change or normalize spacing, tabs, newlines, etc.
- **Do Not Skip Any Modifications:** Include every single changed section, even if it appears trivial, invalid, or incomplete.
- **New Code Additions:** If you must insert code that was not previously present in the original file, pair the old section with the expanded new content.
- **Removals:** If you must remove some code, pair the removed section with the expanded content.

Failure to follow these instructions or use the specified format will result in an incorrect response!"#.to_string();
        prompt
    }

    pub fn followup_prompt(error_message: &String) -> String {
        let prompt = r#"{error_message}

1. List potential reasons why the specified sections couldn't be found.
2. Rewrite the missing sections: Break down each large section into smaller components. 
If there are multiple functions in one section, create individual sections for each function to improve clarity.
3. Copy the correct sections: For sections that are correct, replicate them exactly as they are.
4. Use the hints: Follow any hints provided to identify the precise location for the revised code sections.
5. Maintain the original output format: Ensure your output format mirrors the initial structure. Replace [Modified code section] with the actual modified code as follows:
## Output Format:
### Original Section (to be replaced)
```
[Original code section]
```
### Modified Section (to replace with)
```
[Modified code section]
```"#.to_string();
        prompt.replace("{error_message}", error_message)
    }

    pub async fn parse_message(
        gcx: Arc<ARwLock<GlobalContext>>,
        content: &str,
        filename: &PathBuf,
    ) -> Result<Vec<DiffChunk>, String> {
        let sections = get_edit_sections(content);
        if sections.is_empty() {
            warn!("no sections found, probably an empty diff");
            return Ok(vec![]);
        }
        let diff_blocks = sections_to_diff_blocks(gcx, &sections, &filename).await?;
        let chunks = diff_blocks_to_diff_chunks(&diff_blocks)
            .into_iter()
            .unique()
            .collect::<Vec<_>>();
        Ok(chunks)
    }
}
