use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use hashbrown::HashMap;
use itertools::Itertools;

use crate::call_validation::DiffChunk;
use crate::files_in_workspace::read_file_from_disk;
use crate::privacy::PrivacySettings;
use crate::tools::tool_patch_aux::diff_structs::{diff_blocks_to_diff_chunks, DiffBlock, DiffLine, LineType};

#[derive(Clone, Debug)]
pub struct Edit {
    hunk: Vec<String>,
}

fn process_fenced_block(lines: &[&str], start_line_num: usize) -> (usize, Vec<Edit>) {
    let mut line_num = start_line_num;
    while line_num < lines.len() {
        if lines[line_num].starts_with("```") {
            break;
        }
        line_num += 1;
    }

    let mut block: Vec<&str> = lines[start_line_num..line_num].to_vec();
    block.push("@@ @@");

    if block[0].starts_with("--- ") && block[1].starts_with("+++ ") {
        block = block[2..].to_vec();
    }

    let mut edits = Vec::new();
    let mut hunk = Vec::new();
    for line in block {
        hunk.push(line.to_string());
        if line.len() < 2 {
            continue;
        }

        if line.starts_with("+++ ")
            && hunk.len() >= 3
            && hunk[hunk.len() - 2].starts_with("--- ") {
            if hunk[hunk.len() - 3] == "\n" {
                hunk.truncate(hunk.len() - 3);
            } else {
                hunk.truncate(hunk.len() - 2);
            }

            edits.push(Edit {
                hunk: hunk.clone(),
            });
            hunk.clear();
            continue;
        }

        let op = line.chars().next().unwrap();
        if op == '-' || op == '+' {
            continue;
        }
        if op != '@' {
            continue;
        }
        if hunk.len() <= 1 {
            hunk.clear();
            continue;
        }

        hunk.pop();
        edits.push(Edit {
            hunk: hunk.clone(),
        });
        hunk.clear();
    }

    (line_num + 1, edits)
}

fn get_edit_hunks(content: &str) -> Vec<Edit> {
    let lines: Vec<&str> = content.lines().collect();
    let mut line_num = 0;
    let mut edits: Vec<Edit> = vec![];

    while line_num < lines.len() {
        while line_num < lines.len() {
            let line = lines[line_num];
            if line.starts_with("```diff") {
                let (new_line_num, these_edits) = process_fenced_block(&lines, line_num + 1);
                line_num = new_line_num;
                edits.extend(these_edits);
                break;
            }
            line_num += 1;
        }
    }
    edits
}

async fn edit_hunks_to_diff_blocks(
    edits: &Vec<Edit>,
    filename: &PathBuf,
    privacy_settings: Arc<PrivacySettings>,
) -> Result<Vec<DiffBlock>, String> {
    let mut diff_blocks = vec![];
    let mut files_to_filelines = HashMap::new();
    for (idx, edit) in edits.iter().enumerate() {
        let action = "edit".to_string();
        let file_lines = files_to_filelines
            .entry(filename.clone())
            .or_insert(Arc::new(read_file_from_disk(privacy_settings.clone(), &filename)
                .await
                .map(
                    |x| x
                        .lines()
                        .into_iter()
                        .map(|x| {
                            if let Some(stripped_row) = x.to_string()
                                .replace("\r\n", "\n")
                                .strip_suffix("\n") {
                                stripped_row.to_string()
                            } else {
                                x.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                )?));
        let mut block_has_minus_plus = false;
        let mut current_lines = vec![];
        let has_any_line_no_leading_space = edit.hunk.iter().any(|x| !x.starts_with(" "));
        for line in edit.hunk.iter() {
            if line.starts_with("-") || line.starts_with("+") {
                let is_plus = line.starts_with("+");
                current_lines.push(DiffLine {
                    line: line[1..].to_string(),
                    line_type: if is_plus { LineType::Plus } else { LineType::Minus },
                    file_line_num_idx: None,
                    correct_spaces_offset: None,
                });
                block_has_minus_plus = true;
            } else {
                if block_has_minus_plus {
                    diff_blocks.push(DiffBlock {
                        file_name_before: filename.clone(),
                        file_name_after: filename.clone(),
                        action: action.clone(),
                        file_lines: file_lines.clone(),
                        hunk_idx: idx,
                        diff_lines: current_lines.clone(),
                    });
                    block_has_minus_plus = false;
                    current_lines.clear();
                }
                current_lines.push(DiffLine {
                    line: if !has_any_line_no_leading_space && line.starts_with(" ") {
                        line[1..].to_string()
                    } else {
                        line.clone()
                    },
                    line_type: LineType::Space,
                    file_line_num_idx: None,
                    correct_spaces_offset: None,
                })
            }
        }
        if !current_lines.is_empty() {
            diff_blocks.push(DiffBlock {
                file_name_before: filename.clone(),
                file_name_after: filename.clone(),
                action: action.clone(),
                file_lines: file_lines.clone(),
                hunk_idx: idx,
                diff_lines: current_lines.clone(),
            });
        }
    }
    Ok(diff_blocks)
}

fn search_diff_block_text_location(diff_blocks: &mut Vec<DiffBlock>) {
    for i in 0..diff_blocks.len() {
        let mut blocks_to_search = diff_blocks
            .iter_mut()
            .filter(|x| x.hunk_idx == i)
            .collect::<VecDeque<_>>();
        if blocks_to_search.is_empty() {
            continue;
        }

        let mut file_line_start_offset: usize = 0;
        while let Some(diff_block) = blocks_to_search.pop_front() {
            let mut diff_line_start_offset: usize = 0;
            while diff_line_start_offset <= diff_block.diff_lines.len() {
                let mut found = false;
                for diff_line_span_size in (1..diff_block.diff_lines.len() - diff_line_start_offset + 1).rev() {
                    let span = &diff_block.diff_lines[diff_line_start_offset..diff_line_start_offset + diff_line_span_size];
                    let diff_lines_span = span
                        .iter()
                        .map(|x| &x.line)
                        .map(|x| x.trim_start().to_string())
                        .collect::<Vec<_>>();
                    if span.iter().any(|x| x.line_type == LineType::Plus)
                        || diff_line_span_size >= diff_block.file_lines.len() {
                        continue;
                    }
                    for file_line_idx in file_line_start_offset..=diff_block.file_lines.len() - diff_line_span_size {
                        let file_lines_span = diff_block.file_lines[file_line_idx..file_line_idx + diff_line_span_size]
                            .iter()
                            .map(|x| x.trim_start().to_string())
                            .collect::<Vec<_>>();
                        if file_line_idx > file_line_start_offset &&
                            (file_lines_span.is_empty() || diff_lines_span.iter().all(|c| c == "")) {
                            continue;
                        }
                        if file_lines_span == diff_lines_span {
                            for (idx, line) in diff_block.diff_lines[diff_line_start_offset..diff_line_start_offset + diff_line_span_size]
                                .iter_mut()
                                .enumerate() {
                                let file_lines_idents_count = diff_block.file_lines[file_line_idx + idx]
                                    .chars()
                                    .take_while(|x| x.eq(&' '))
                                    .join("")
                                    .len() as i64;
                                let diff_lines_idents_count = line.line
                                    .chars()
                                    .take_while(|x| x.eq(&' '))
                                    .join("")
                                    .len() as i64;
                                line.file_line_num_idx = Some(file_line_idx + idx);
                                line.correct_spaces_offset = Some(file_lines_idents_count - diff_lines_idents_count);
                            }
                            diff_line_start_offset = diff_line_start_offset + diff_line_span_size;
                            file_line_start_offset = file_line_idx + diff_line_span_size;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        break;
                    }
                }
                if !found {
                    diff_line_start_offset += 1;
                }
            }
        }
    }
}

// Step 1. Fix idents using correct_spaces_offset
// Step 2. If the first line is not found, and it is a `+` type then set the index to 0 
// Step 3. Fix missing `+` lines. If line is without `+` symbol and is file line index is not found then consider it a `+` line (except the first line)
// Step 4. Fix missing `-` lines. If line is without `-` symbol and file index is found and the nearest `+` line is quite similar then consider it as a `-` line
// Step 5. Fill out all non-found file indexes using the last one found.
fn normalize_diff_block(diff_block: &mut DiffBlock) -> Result<(), String> {
    if diff_block.diff_lines.is_empty() {
        return Ok(());
    }

    // Step 1
    for diff_line in diff_block.diff_lines.iter_mut() {
        if let Some(correct_spaces_offset) = diff_line.correct_spaces_offset {
            if correct_spaces_offset > 0 {
                diff_line.line.insert_str(0, &" ".repeat(correct_spaces_offset as usize));
            } else if correct_spaces_offset < 0 {
                diff_line.line = diff_line.line.chars().skip(correct_spaces_offset.abs() as usize).join("");
            }
        }
    }

    // Step 2
    match diff_block.diff_lines.get_mut(0) {
        Some(line) => {
            if line.line_type == LineType::Plus && line.file_line_num_idx.is_none() {
                line.file_line_num_idx = Some(0);
            }
        }
        None => {}
    };
    diff_block.diff_lines = diff_block
        .diff_lines
        .iter()
        .skip_while(|x| x.line_type == LineType::Space && x.file_line_num_idx.is_none())
        .cloned()
        .collect::<Vec<_>>();

    // Step 3 (doesn't work well enough)
    // for diff_line in diff_block.diff_lines.iter_mut() {
    //     if diff_line.line_type == LineType::Space || diff_line.file_line_num_idx.is_none() {
    //         diff_line.line_type = LineType::Plus;
    //     }
    // }

    // Step 4
    let diff_lines_copy = diff_block.diff_lines.clone();
    for (idx, diff_line) in diff_block.diff_lines.iter_mut().enumerate() {
        if diff_line.line_type == LineType::Space
            && diff_line.file_line_num_idx.is_some()
            && idx < diff_lines_copy.len() - 1 {
            let nearest_plus_diff_line = match diff_lines_copy[idx + 1..]
                .iter()
                .find(|x| x.line_type == LineType::Plus) {
                Some(item) => item,
                None => {
                    continue
                }
            };
            if diff_line.line == nearest_plus_diff_line.line {
                diff_line.line_type = LineType::Minus;
            }
        }
    }

    // Step 5
    let mut last_file_line_num_idx = None;
    for diff_line in diff_block.diff_lines.iter_mut() {
        if diff_line.file_line_num_idx.is_some() {
            last_file_line_num_idx = diff_line.file_line_num_idx.map(|x| x + 1);
        } else {
            diff_line.file_line_num_idx = last_file_line_num_idx;
        }
    }

    // Validation step
    let non_found_lines = diff_block.diff_lines
        .iter()
        .filter(|x| x.line_type != LineType::Space && x.file_line_num_idx.is_none())
        .map(|x| format!("{}{}", x.line_type, x.line))
        .collect::<Vec<_>>();
    if !non_found_lines.is_empty() {
        return Err(format!(
            "blocks of code signed with '-' weren't found in a file\n{}\n",
            non_found_lines.join("\n")
        ));
    }

    Ok(())
}

pub struct UnifiedDiffParser {}

impl UnifiedDiffParser {
    pub fn prompt(
        workspace_projects_dirs: Vec<String>
    ) -> String {
        assert_eq!(workspace_projects_dirs.is_empty(), false);
        let prompt = r#"YOU ARE THE WORLD'S LEADING AUTO CODING ASSISTANT.
You will receive some file containing code along with one or several modified sections.
Your task is to generate a unified diff in a specified format, comparing the original file to the updated portions.

### UNIFIED DIFF FORMATTING RULES

## Rules to generate correct diffs:
- Fence the diff with "```diff" and "```".
- Return edits similar to unified diffs that `diff -U2` would produce.
- Don't include line numbers like `diff -U2` does. The user's patch tool doesn't need them.
- Copy a few lines from the original file and paste them before the `-` and `+` lines, otherwise the diff will be incorrect.
- Make sure you mark all new or modified lines with `+`.
- Make sure you include and mark all lines that need to be removed as `-` lines.
- Rewrite the whole blocks of code instead of making multiple small changes.
- Use filenames from the user as given, don't change them.
- When editing a function, method, loop, etc. use a hunk to replace the *entire* code block.

## Format example for the task: "Replace is_prime with a call to sympy"
```diff
--- %FIRST_WORKSPACE_PROJECT_DIR%/test.py
+++ %FIRST_WORKSPACE_PROJECT_DIR%/test.py
@@ ... @@
+import sympy
+
@@ ... @@
-def is_prime(x):
-    if x < 2:
-        return False
-    for i in range(2,
-                  int(math.sqrt(x)) + 1):
-        if x % i == 0:
-            return False
-    return True
@@ ... @@
-@app.route('/prime/<int:n>')
-def nth_prime(n):
-    count = 0
-    num = 1
-    while count < n:
-        num += 1
-        if is_prime(num):
-            count += 1
-    return str(num)
+@app.route('/prime/<int:n>')
+def nth_prime(n):
+    count = 0
+    num = 1
+    while count < n:
+        num += 1
+        if sympy.isprime(num):
+            count += 1
+    return str(num)
@@ ... @@
+
+def nth_prime_test(n):
+    pass
```

Do not forget to place `+` and `-` markings when it's needed!"#.to_string();
        prompt
            .replace("%WORKSPACE_PROJECTS_DIRS%", &workspace_projects_dirs.join("\n"))
            .replace("%FIRST_WORKSPACE_PROJECT_DIR%", &workspace_projects_dirs[0])
    }

    pub async fn parse_message(
        content: &str,
        filename: &PathBuf,
        privacy_settings: Arc<PrivacySettings>,
    ) -> Result<Vec<DiffChunk>, String> {
        let edits = get_edit_hunks(content);
        let mut diff_blocks = edit_hunks_to_diff_blocks(&edits, &filename, privacy_settings).await?;
        search_diff_block_text_location(&mut diff_blocks);
        for block in diff_blocks.iter_mut() {
            match normalize_diff_block(block) {
                Ok(_) => {}
                Err(err) => {
                    return Err(err);
                }
            };
        }
        let filtered_blocks = diff_blocks
            .into_iter()
            .filter(|x| if x.action == "edit" {
                x.diff_lines
                    .iter()
                    .any(|x| x.line_type == LineType::Plus || x.line_type == LineType::Minus)
            } else { true })
            .collect::<Vec<_>>();
        let chunks = diff_blocks_to_diff_chunks(&filtered_blocks)
            .into_iter()
            .unique()
            .collect::<Vec<_>>();
        Ok(chunks)
    }
}
