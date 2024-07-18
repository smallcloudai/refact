use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use hashbrown::HashMap;
use itertools::Itertools;

use crate::call_validation::DiffChunk;
use crate::files_in_workspace::read_file_from_disk;

#[derive(Clone, Debug)]
struct Edit {
    path: Option<String>,
    hunk: Vec<String>,
}

#[derive(Clone, Eq, PartialEq)]
enum LineType {
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
struct DiffLine {
    line: String,
    line_type: LineType,
    file_line_num_idx: Option<usize>,
    correct_spaces_offset: Option<i64>,
}

#[derive(Clone, Eq, PartialEq)]
struct DiffBlock {
    file_name: PathBuf,
    diff_lines: Vec<DiffLine>,
    hunk_idx: usize,
    file_lines: Arc<Vec<String>>,
}

impl DiffBlock {
    pub fn display(&self) -> String {
        let mut output = format!(
            "--- {:?}\n+++ {:?}\n@@ ... @@\n",
            &self.file_name,
            &self.file_name
        );
        for line in self.diff_lines.iter() {
            output.push_str(&format!("{}{}", line.line_type, line.line));
        }
        output
    }
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

    let mut fname = None;
    if block[0].starts_with("--- ") && block[1].starts_with("+++ ") {
        fname = Some(block[1][4..].trim().to_string());
        block = block[2..].to_vec();
    }

    let mut edits = Vec::new();
    let mut keeper = false;
    let mut hunk = Vec::new();

    for line in block {
        hunk.push(line.to_string());
        if line.len() < 2 {
            continue;
        }

        if line.starts_with("+++ ") && hunk[hunk.len() - 2].starts_with("--- ") {
            if hunk[hunk.len() - 3] == "\n" {
                hunk.truncate(hunk.len() - 3);
            } else {
                hunk.truncate(hunk.len() - 2);
            }

            edits.push(Edit {
                path: fname.clone(),
                hunk: hunk.clone(),
            });
            hunk.clear();
            keeper = false;

            fname = Some(line[4..].trim().to_string());
            continue;
        }

        let op = line.chars().next().unwrap();
        if op == '-' || op == '+' {
            keeper = true;
            continue;
        }
        if op != '@' {
            continue;
        }
        if !keeper {
            hunk.clear();
            continue;
        }

        hunk.pop();
        edits.push(Edit {
            path: fname.clone(),
            hunk: hunk.clone(),
        });
        hunk.clear();
        keeper = false;
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

async fn edit_hunks_to_diff_blocks(edits: &Vec<Edit>) -> Result<Vec<DiffBlock>, String> {
    let mut diff_blocks = vec![];
    let mut files_to_filelines = HashMap::new();
    for (idx, edit) in edits.iter().enumerate() {
        let path = match edit.path.clone() {
            Some(p) => PathBuf::from(p.clone()),
            None => {
                return Err(format!("cannot get a correct file name from the diff chunk:\n{edit:?}\n"));
            }
        };
        let file_lines = files_to_filelines
            .entry(path.clone())
            .or_insert(Arc::new(read_file_from_disk(&path)
                .await
                .map_err(|e| {
                    format!("couldn't read file from the diff chunk: {:?}. Error: {}", &path, e)
                })
                .map(
                    |x| x
                        .lines()
                        .into_iter()
                        .map(|x| x.to_string().trim_end().to_string())
                        .collect::<Vec<_>>()
                )?));

        let mut block_has_minus_plus = false;
        let mut current_lines = vec![];
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
                        file_name: path.clone(),
                        file_lines: file_lines.clone(),
                        hunk_idx: idx,
                        diff_lines: current_lines.clone(),
                    });
                    block_has_minus_plus = false;
                    current_lines.clear();
                }
                current_lines.push(DiffLine {
                    line: if line.starts_with(" ") { line[1..].to_string() } else { line.clone() },
                    line_type: LineType::Space,
                    file_line_num_idx: None,
                    correct_spaces_offset: None,
                })
            }
        }
        if !current_lines.is_empty() {
            diff_blocks.push(DiffBlock {
                file_name: path.clone(),
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
                    if span.iter().any(|x| x.line_type == LineType::Plus) {
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
// Step 2. If non-found is the first line, and it is a `+` type then set the 0 index
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
            "some diff block lines weren't found in the file:\n{:?}",
            non_found_lines
        ));
    }

    return Ok(());
}

fn diff_blocks_to_diff_chunks(diff_blocks: &Vec<DiffBlock>) -> Vec<DiffChunk> {
    diff_blocks
        .iter()
        .map(|block| {
            let useful_block_lines = block
                .diff_lines
                .iter()
                .filter(|x| x.line_type != LineType::Space)
                .collect::<Vec<_>>();
            DiffChunk {
                file_name: block.file_name.to_string_lossy().to_string(),
                file_action: "edit".to_string(),
                line1: useful_block_lines
                    .iter()
                    .map(|x| x.file_line_num_idx.clone().expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 1)
                    .min()
                    .expect("All empty diff blocks should be filtered before"),
                line2: useful_block_lines
                    .iter()
                    .map(|x| {
                        if x.line_type == LineType::Plus {
                            x.file_line_num_idx.clone().expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 1
                        } else {
                            x.file_line_num_idx.clone().expect("All file_line_num_idx must be filled to this moment in the `normalize_diff_block` func") + 2
                        }
                    })
                    .max()
                    .expect("All empty diff blocks should be filtered before"),
                lines_remove: useful_block_lines
                    .iter()
                    .filter(|x| x.line_type == LineType::Minus)
                    .map(|x| format!("{}\n", x.line.clone()))
                    .join(""),
                lines_add: useful_block_lines
                    .iter()
                    .filter(|x| x.line_type == LineType::Plus)
                    .map(|x| format!("{}\n", x.line.clone()))
                    .join(""),
            }
        })
        .collect()
}


pub struct UnifiedDiffFormat {}

impl UnifiedDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is to create a unified diff format output based on the provided task and files.

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
    - Review the tasks and files provided
    - Use extra context (list of related symbols signatures) if it's given to make correct changes
    - Identify the specific changes required
    - Use chain of thoughts to make sure nothing will be missed
    - Explain every change in all files before making the diff
    - For each explanation describe whether `-` or `+` should be used
    - Pretend that you're applying the generated diff and try to asses it correctness
    - Assess the generated diff, especially its format validity (`+` and `-` symbols are in the right places and nothing is missing)

2. **Generate Diff:**
    - Fence the diff with "```diff" and "```"
    - Make changes to all given files
    - Return edits similar to unified diffs that `diff -U0` would produce.
    - Include the first 2 lines with the real file paths which were given before
    - Don't include timestamps with the file paths.
    - Start each hunk of changes with a `@@ ... @@` line.
    - Don't include line numbers like `diff -U0` does. The user's patch tool doesn't need them.
    - The user's patch tool needs CORRECT patches that apply cleanly against the current contents of the file.
    - Make sure you mark all new or modified lines with `+`.
    - Think carefully and make sure you include and mark all lines that need to be removed or changed as `-` lines.
    - Do not forget to remove old lines of the code if you are fixing some code
    - Don't leave out any lines or the diff patch won't apply correctly.
    - Only output hunks that specify changes with `+` or `-` lines.
    - Output hunks in whatever order makes the most sense.
    - When editing a function, method, loop, etc. use a hunk to replace the *entire* code block.
    - Delete the entire existing version with `-` lines and then add a new, updated version with `+` lines. This will help you generate correct code and correct diffs.
    - To move code within a file, use 2 hunks: 1 to delete it from its current location, 1 to insert it in the new location

There is a unified diff format example for the task: "Replace is_prime with a call to sympy"
```diff
--- /home/mathweb/flask/app.py
+++ /home/mathweb/flask/app.py
@@ ... @@
 import some_module
 
-class MathWeb:
+import sympy
+
+class MathWeb:
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
 ):
     pass
+
+def nth_prime_test(n):
+    pass
```"#.to_string()
    }

    pub async fn parse_message(
        content: &str,
    ) -> Result<Vec<DiffChunk>, String> {
        let edits = get_edit_hunks(content);
        let mut diff_blocks = edit_hunks_to_diff_blocks(&edits).await?;
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
            .filter(|x| x.diff_lines
                .iter()
                .any(|x| x.line_type == LineType::Plus || x.line_type == LineType::Minus))
            .collect::<Vec<_>>();
        let chunks = diff_blocks_to_diff_chunks(&filtered_blocks)
            .into_iter()
            .unique()
            .collect::<Vec<_>>();
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use itertools::Itertools;

    use crate::at_tools::att_patch::unified_diff_format::UnifiedDiffFormat;
    use crate::call_validation::DiffChunk;
    use crate::diffs::{apply_diff_chunks_to_text, fuzzy_results_into_state_vector};

    fn apply_diff(path: &String, chunks: &Vec<DiffChunk>) -> (String, String) {
        let text = std::fs::read_to_string(PathBuf::from(path)).unwrap();
        let (changed_text, fuzzy_results) = apply_diff_chunks_to_text(
            &text,
            chunks.iter().enumerate().collect::<Vec<_>>(),
            vec![],
            1,
        );
        let state = fuzzy_results_into_state_vector(&fuzzy_results, chunks.len());
        assert!(state.iter().all(|x| *x == 1));
        (text, changed_text)
    }

    #[tokio::test]
    async fn test_empty_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
```
Another text"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_2() {
        let input = r#""#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_3() {
        let input = r#"Initial text
```diff
Another text"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_4() {
        let input = r#"Initial text
```"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_empty_5() {
        let input = r#"Initial text
```diff
some invalid text
```
```
```diff"#;
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_simple_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-class Frog:
+class AnotherFrog:
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

class AnotherFrog:
    def __init__(self, x, y, vx, vy):"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 5,
                line2: 6,
                lines_remove: "class Frog:\n".to_string(),
                lines_add: "class AnotherFrog:\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(6).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_simple_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
 DT = 0.01
 
 
-class Frog:
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

    def __init__(self, x, y, vx, vy):"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 5,
                line2: 6,
                lines_remove: "class Frog:\n".to_string(),
                lines_add: "".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
        
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(5).join("\n");
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_simple_hunk_3() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
 DT = 0.01
 
 class Frog:
+    # Frog class description
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

class Frog:
    # Frog class description
    def __init__(self, x, y, vx, vy):"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 6,
                line2: 6,
                lines_remove: "".to_string(),
                lines_add: "    # Frog class description\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(7).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_simple_hunk_4() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
+    # Frog class description
```
Another text"#;
        let gt_changed_text = r#"    # Frog class description
import numpy as np

DT = 0.01"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 1,
                line2: 1,
                lines_remove: "".to_string(),
                lines_add: "    # Frog class description\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(4).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_simple_hunk_5() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-    def jump(self, pond_width, pond_height):
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

        self.x += self.vx * DT
        self.y += self.vy * DT"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 22,
                line2: 23,
                lines_remove: "    def jump(self, pond_width, pond_height):\n".to_string(),
                lines_add: "".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(23).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_simple_hunk_6() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@

-    def jump(self, pond_width, pond_height):
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

        self.x += self.vx * DT
        self.y += self.vy * DT"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 22,
                line2: 23,
                lines_remove: "    def jump(self, pond_width, pond_height):\n".to_string(),
                lines_add: "".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(23).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }
    #[tokio::test]
    async fn test_complex_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
     def bounce_off_banks(self, pond_width, pond_height):
         if self.x < 0:
             self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
             self.vy = np.abs(self.vy)
         elif self.y > pond_height:
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;
        let gt_changed_text = r#"import numpy as np

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        # test1:
        elif self.x > pond:
            # what is that?
            pass
        if self.y > 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vx = -np.abs(self.vy)"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 18,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 21,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(22).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_complex_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
     def bounce_off_banks(self, pond_width, pond_height):
         if self.x < 0:
             self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 18,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 21,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_3() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
     def bounce_off_banks(self, pond_width, pond_height):
         if self.x < 0:
             self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        # test1:
+        elif self.x > pond:
-            self.vx = -np.abs(self.vx)
-        if self.y < 0:
+            # what is that?
+            pass
+        if self.y > 0:
```
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-            self.vy = -np.abs(self.vy)
+            self.vx = -np.abs(self.vy)
```
Another text"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 18,
                lines_remove: "        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n".to_string(),
                lines_add: "        # test1:\n        elif self.x > pond:\n            # what is that?\n            pass\n        if self.y > 0:\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 20,
                line2: 21,
                lines_remove: "            self.vy = -np.abs(self.vy)\n".to_string(),
                lines_add: "            self.vx = -np.abs(self.vy)\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
    }

    #[tokio::test]
    async fn test_complex_hunk_4() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
         self.y = np.clip(self.y, 0, pond_height)
+        # extra row 1
+        # extra row 2
+        # extra row 3
--- tests/emergency_frog_situation/frog.py
+++ tests/emergency_frog_situation/frog.py
@@ ... @@
-import numpy as np
+import numpy as np
+# extra row 1
+# extra row 2
+# extra row 3
```
Another text"#;
        let gt_changed_text = r#"import numpy as np
# extra row 1
# extra row 2
# extra row 3

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)
        # extra row 1
        # extra row 2
        # extra row 3
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 28,
                line2: 28,
                lines_remove: "".to_string(),
                lines_add: "        # extra row 1\n        # extra row 2\n        # extra row 3\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 1,
                line2: 2,
                lines_remove: "import numpy as np\n".to_string(),
                lines_add: "import numpy as np\n# extra row 1\n# extra row 2\n# extra row 3\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/frog.py".to_string(),
            &result,
        );

        assert_eq!(result, gt_result);
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_1() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/set_as_avatar.py
+++ tests/emergency_frog_situation/set_as_avatar.py
@@ ... @@
     """
 
     def __init__(self, x, y, vx, vy):
+        # extra row 1
+        # extra row 2
+        # extra row 3
```
Another text"#;
        let gt_changed_text = r#"# Picking up context, goal in this file:
# - goto parent class, two times
# - dump parent class

import frog

X,Y = 50, 50
W = 100
H = 100


# This this a comment for the Toad class, above the class
class Toad(frog.Frog):
    def __init__(self, x, y, vx, vy):
        super().__init__(x, y, vx, vy)
        self.name = "Bob"


class EuropeanCommonToad(frog.Frog):
    """
    This is a comment for EuropeanCommonToad class, inside the class
    """

    def __init__(self, x, y, vx, vy):
        # extra row 1
        # extra row 2
        # extra row 3
        super().__init__(x, y, vx, vy)
        self.name = "EU Toad""#;

        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/set_as_avatar.py".to_string(),
                file_action: "edit".to_string(),
                line1: 25,
                line2: 25,
                lines_remove: "".to_string(),
                lines_add: "        # extra row 1\n        # extra row 2\n        # extra row 3\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/set_as_avatar.py".to_string(),
            &result,
        );
        let cropped_text = changed_text.lines().take(29).join("\n");

        assert_eq!(result, gt_result);
        assert_eq!(cropped_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_2() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
     frog2.jump()
 
-    # Third jump
+    # New Comment
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()

    # Second jump
    frog1.jump()
    frog2.jump()

    # New Comment
    frog1.jump()
    frog2.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 16,
                line2: 17,
                lines_remove: "    # Third jump\n".to_string(),
                lines_add: "    # New Comment\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );

        assert_eq!(result, gt_result);
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_3() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
     frog1.jump()
     frog2.jump()
 
     # Second jump
+    frog3 = Frog()
     frog1.jump()
     frog2.jump()
+    frog3.jump()
 
-    # Third jump
+    # Third extra jump
     frog1.jump()
-    frog2.jump()
+    frog2.jump()
+    frog3.jump()
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()

    # Second jump
    frog3 = Frog()
    frog1.jump()
    frog2.jump()
    frog3.jump()

    # Third extra jump
    frog1.jump()
    frog2.jump()
    frog3.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 13,
                line2: 13,
                lines_remove: "".to_string(),
                lines_add: "    frog3 = Frog()\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 15,
                line2: 15,
                lines_remove: "".to_string(),
                lines_add: "    frog3.jump()\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 16,
                line2: 17,
                lines_remove: "    # Third jump\n".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 18,
                line2: 19,
                lines_remove: "    frog2.jump()\n".to_string(),
                lines_add: "    frog2.jump()\n    frog3.jump()\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);

        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_4() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
     frog1.jump()
     frog2.jump()
-    # Third jump
+    # Third extra jump
     frog1.jump()
-    frog2.jump()
+    frog2.jump()
+    frog3.jump()
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()

    # Second jump
    frog1.jump()
    frog2.jump()

    # Third extra jump
    frog1.jump()
    frog2.jump()
    frog3.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 16,
                line2: 17,
                lines_remove: "    # Third jump\n".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 18,
                line2: 19,
                lines_remove: "    frog2.jump()\n".to_string(),
                lines_add: "    frog2.jump()\n    frog3.jump()\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );

        assert_eq!(result, gt_result);
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_5() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
    frog1.jump()
    frog2.jump()
+    # Third extra jump
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()
    # Third extra jump

    # Second jump
    frog1.jump()
    frog2.jump()

    # Third jump
    frog1.jump()
    frog2.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 11,
                line2: 11,
                lines_remove: "".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );

        assert_eq!(result, gt_result);
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_6() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
    invalid row
    frog2.jump()
+    # Third extra jump
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()
    # Third extra jump

    # Second jump
    frog1.jump()
    frog2.jump()

    # Third jump
    frog1.jump()
    frog2.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 11,
                line2: 11,
                lines_remove: "".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);

        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_7() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
    invalid row
-    frog2.jump()
+    # Third extra jump
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    # Third extra jump

    # Second jump
    frog1.jump()
    frog2.jump()

    # Third jump
    frog1.jump()
    frog2.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 10,
                line2: 11,
                lines_remove: "    frog2.jump()\n".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);

        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn test_ambiguous_hunk_8() {
        let input = r#"Initial text
```diff
--- tests/emergency_frog_situation/holiday.py
+++ tests/emergency_frog_situation/holiday.py
@@ ... @@
    frog1 = frog.Frog()
    frog2 = frog.Frog()
    frog2.jump()
+    # Third extra jump
```
Another text"#;
        let gt_changed_text = r#"import frog


if __name__ == __main__:
    frog1 = frog.Frog()
    frog2 = frog.Frog()

    # First jump
    frog1.jump()
    frog2.jump()
    # Third extra jump

    # Second jump
    frog1.jump()
    frog2.jump()

    # Third jump
    frog1.jump()
    frog2.jump()

    # Forth jump
    frog1.jump()
    frog2.jump()
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 11,
                line2: 11,
                lines_remove: "".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        let (_, changed_text) = apply_diff(
            &"./tests/emergency_frog_situation/holiday.py".to_string(),
            &result,
        );

        assert_eq!(result, gt_result);
        assert_eq!(changed_text, gt_changed_text);
    }

    #[tokio::test]
    async fn info_test() {
        let input = r#"Certainly! I'll analyze the task and create a unified diff for each file. Let's go through the changes step by step.

1. Changes in frog.py:
   - Replace the `Frog` class with the `Bird` class
   - Change `pond_width` and `pond_height` to `sky_width` and `sky_height`
   - Rename the `jump` method to `fly`

2. Changes in holiday.py:
   - Replace `frog.Frog()` with `frog.Bird()`
   - Change `jump()` calls to `fly()`

3. Changes in jump_to_conclusions.py:
   - Replace `frog.Frog` with `frog.Bird`
   - Change `p.jump(W, H)` to `p.fly(W, H)`
   - Update the message "Jump To Conclusions!" to "Fly To Conclusions!"

Now, let's create the unified diff for each file:

--- /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/frog.py
+++ /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/frog.py
@@ ... @@
 
 DT = 0.01
 
-class Frog:
+class Bird:
     def __init__(self, x, y, vx, vy):
         self.x = x
         self.y = y
         self.vx = vx
         self.vy = vy
 
-    def bounce_off_banks(self, pond_width, pond_height):
+    def bounce_off_banks(self, sky_width, sky_height):
         if self.x < 0:
             self.vx = np.abs(self.vx)
-        elif self.x > pond_width:
+        elif self.x > sky_width:
             self.vx = -np.abs(self.vx)
         if self.y < 0:
             self.vy = np.abs(self.vy)
-        elif self.y > pond_height:
+        elif self.y > sky_height:
             self.vy = -np.abs(self.vy)
 
-    def jump(self, pond_width, pond_height):
+    def fly(self, sky_width, sky_height):
         self.x += self.vx * DT
         self.y += self.vy * DT
-        self.bounce_off_banks(pond_width, pond_height)
-        self.x = np.clip(self.x, 0, pond_width)
-        self.y = np.clip(self.y, 0, pond_height)
+        self.bounce_off_banks(sky_width, sky_height)
+        self.x = np.clip(self.x, 0, sky_width)
+        self.y = np.clip(self.y, 0, sky_height)

--- /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/holiday.py
+++ /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/holiday.py
@@ ... @@
 
 
 if __name__ == __main__:
-    frog1 = frog.Frog()
-    frog2 = frog.Frog()
+    bird1 = frog.Bird()
+    bird2 = frog.Bird()
 
-    # First jump
-    frog1.jump()
-    frog2.jump()
+    # First fly
+    bird1.fly()
+    bird2.fly()
 
-    # Second jump
-    frog1.jump()
-    frog2.jump()
+    # Second fly
+    bird1.fly()
+    bird2.fly()
 
-    # Third jump
-    frog1.jump()
-    frog2.jump()
+    # Third fly
+    bird1.fly()
+    bird2.fly()
 
-    # Forth jump
-    frog1.jump()
-    frog2.jump()
+    # Fourth fly
+    bird1.fly()
+    bird2.fly()

--- /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py
+++ /home/svakhreev/projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py
@@ ... @@
 H = 480
 
 
-def draw_hello_frog(
+def draw_hello_bird(
     screen: pygame.Surface,
     message: str,
     color: Tuple[int, int, int] = (0, 255, 255),
@@ -17,7 +17,7 @@ def draw_hello_frog(
 
 
 creatures = [
-    frog.Frog(
+    frog.Bird(
         np.random.uniform(0, W),
         np.random.uniform(0, H),
         np.random.uniform(-W/10, H/10),
@@ -34,11 +34,11 @@ def main_loop():
         screen.fill((0, 0, 0))
         for p in creatures:
             pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)
-        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))
+        draw_hello_bird(screen, "Fly To Conclusions!", (0, 200, 0))
         pygame.display.flip()
         pygame.time.Clock().tick(60)
-        p: frog.Frog
+        p: frog.Bird
         for p in creatures:
-            p.jump(W, H)
+            p.fly(W, H)
 
 
@@ ... @@
-    pygame.display.set_caption("Pond")
+    pygame.display.set_caption("Sky")
     main_loop()
     pygame.quit()

These unified diffs should correctly apply the required changes to replace the Frog class with a Bird class and update all references accordingly. The changes include renaming methods, updating parameter names, and modifying related text to reflect the switch from frogs to birds.
"#;
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 10,
                line2: 10,
                lines_remove: "".to_string(),
                lines_add: "    # Third extra jump\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        print!("Result: {:?}\n", serde_json::to_string_pretty(&result));
    }
}
