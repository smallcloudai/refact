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
        let input = r#" I'll extract helper functions, add docstrings, and ensure consistent formatting. Here's the refactored version:

```diff
--- /home/svakhreev/projects/smc/refact/self_hosting_machinery/finetune/scripts/finetune_train.py
+++ /home/svakhreev/projects/smc/refact/self_hosting_machinery/finetune/scripts/finetune_train.py
@@ -46,71 +46,114 @@ def _log_everywhere(message):
     traces.log(message)
 
 
-def _build_finetune_config_by_heuristics(run_id: str, finetune_cfg: Dict, model_config: Dict, **kwargs) -> Dict[str, Any]:
-    user_cfg = copy.deepcopy(finetune_train_defaults)
-    user_cfg_nondefault = {}
-    for k, v in kwargs.items():
-        # traces.log("Command line parameter: %s = %s" % (k, v))
-        user_cfg[k] = v
-        if finetune_train_defaults.get(k, 0) != v:
-            user_cfg_nondefault[k] = v
+def _build_finetune_config_by_heuristics(run_id: str, finetune_cfg: Dict, model_config: Dict, **kwargs) -> Dict[str, Any]:
+    """
+    Build the finetune configuration based on heuristics and user-provided parameters.
+    
+    Args:
+        run_id (str): The run identifier.
+        finetune_cfg (Dict): The base finetune configuration.
+        model_config (Dict): The model configuration.
+        **kwargs: Additional keyword arguments for configuration.
+    
+    Returns:
+        Dict[str, Any]: The final finetune configuration.
+    """
+    user_cfg, user_cfg_nondefault = _process_user_config(kwargs)
     cfg_builder = ConfigBuilder(finetune_cfg)
-    # if user_cfg['use_heuristics']:
+    
     if user_cfg['train_steps'] == 0:
-        _log_everywhere("Retrieving dataset length per epoch, it may take a while...")
-        ds_len = get_ds_len_per_epoch(env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id), model_config, cfg_builder)
-        traces.log(f"Dataset length per epoch = {ds_len}")
-        # set_lora_quality_by_heuristics sets inside:
-        # lora_target_modules=[
-        #             "qkv", "out", "mlp",
-        #         ], lora_r=64, lora_alpha=128, lora_dropout=0.01,
-        #             freeze_exceptions=[
-        #                 "wte", "lm_head", "lora"
-        #             ]
-        (cfg_builder
-         .set_batch_size(cfg_builder.cfg['train_batch_size'])
-        #  .set_lora_quality_by_heuristics(ds_len=ds_len, initial_loss=initial_loss)
-         .set_schedule_by_heuristics(ds_len=ds_len)    # analog of set_train_steps + set_lr_decay_steps
-         .set_lora_r(user_cfg['lora_r'])
-         .set_lora_alpha(user_cfg['lora_alpha'])
-         .set_lora_dropout(user_cfg['lora_dropout'])
-         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
-         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
-        #  .set_low_gpu_mem_mode_by_heuristics())
+        _configure_with_heuristics(cfg_builder, run_id, model_config, user_cfg)
     else:
-        _log_everywhere("Using finetune setup parameters")
-        (cfg_builder
-         .set_train_steps(user_cfg['train_steps'])
-         .set_lr_decay_steps(max(user_cfg['lr_decay_steps'], user_cfg['train_steps']))
-         .set_lora_r(user_cfg['lora_r'])
-         .set_lora_alpha(user_cfg['lora_alpha'])
-         .set_lora_dropout(user_cfg['lora_dropout'])
-         .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
-         .set_trainable_embeddings(user_cfg['trainable_embeddings']))
-    (cfg_builder
-        .set_lr(user_cfg['lr'])
-        .set_batch_size(user_cfg['batch_size'])
-        .set_warmup_steps(user_cfg['warmup_num_steps'])
-        # .set_limit_time_seconds(user_cfg['limit_time_seconds'])
-        .set_weight_decay(user_cfg['weight_decay']))
+        _configure_with_user_params(cfg_builder, user_cfg)
+    
+    _set_common_config(cfg_builder, user_cfg)
+    _log_file_types(run_id)
+    _log_config(user_cfg_nondefault, cfg_builder)
+    
+    assert cfg_builder.cfg['train_iters'] % cfg_builder.cfg['test_every'] == 0
+    assert cfg_builder.cfg['save_every'] % cfg_builder.cfg['test_every'] == 0
+    
+    return cfg_builder.cfg
 
+def _process_user_config(kwargs: Dict[str, Any]) -> Tuple[Dict[str, Any], Dict[str, Any]]:
+    """Process user-provided configuration and identify non-default values."""
+    user_cfg = copy.deepcopy(finetune_train_defaults)
+    user_cfg_nondefault = {}
+    for k, v in kwargs.items():
+        user_cfg[k] = v
+        if finetune_train_defaults.get(k, 0) != v:
+            user_cfg_nondefault[k] = v
+    return user_cfg, user_cfg_nondefault
+
+def _configure_with_heuristics(cfg_builder: ConfigBuilder, run_id: str, model_config: Dict, user_cfg: Dict[str, Any]):
+    """Configure the builder using heuristics based on dataset length."""
+    _log_everywhere("Retrieving dataset length per epoch, it may take a while...")
+    ds_len = get_ds_len_per_epoch(env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id), model_config, cfg_builder)
+    traces.log(f"Dataset length per epoch = {ds_len}")
+    
+    (cfg_builder
+     .set_batch_size(cfg_builder.cfg['train_batch_size'])
+     .set_schedule_by_heuristics(ds_len=ds_len)
+     .set_lora_r(user_cfg['lora_r'])
+     .set_lora_alpha(user_cfg['lora_alpha'])
+     .set_lora_dropout(user_cfg['lora_dropout'])
+     .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
+     .set_trainable_embeddings(user_cfg['trainable_embeddings']))
+
+def _configure_with_user_params(cfg_builder: ConfigBuilder, user_cfg: Dict[str, Any]):
+    """Configure the builder using user-provided parameters."""
+    _log_everywhere("Using finetune setup parameters")
+    (cfg_builder
+     .set_train_steps(user_cfg['train_steps'])
+     .set_lr_decay_steps(max(user_cfg['lr_decay_steps'], user_cfg['train_steps']))
+     .set_lora_r(user_cfg['lora_r'])
+     .set_lora_alpha(user_cfg['lora_alpha'])
+     .set_lora_dropout(user_cfg['lora_dropout'])
+     .set_low_gpu_mem_mode(user_cfg['low_gpu_mem_mode'])
+     .set_trainable_embeddings(user_cfg['trainable_embeddings']))
+
+def _set_common_config(cfg_builder: ConfigBuilder, user_cfg: Dict[str, Any]):
+    """Set common configuration parameters."""
+    (cfg_builder
+     .set_lr(user_cfg['lr'])
+     .set_batch_size(user_cfg['batch_size'])
+     .set_warmup_steps(user_cfg['warmup_num_steps'])
+     .set_weight_decay(user_cfg['weight_decay']))
+
+def _log_file_types(run_id: str):
+    """Log the file types for train and test datasets."""
     if dist.get_rank() == 0:
         filetypes_train = count_file_types(env.PERRUN_TRAIN_FILTERED_FILEPATH(run_id))
         filetypes_test = count_file_types(env.PERRUN_TEST_FILTERED_FILEPATH(run_id))
-        traces.log(f'Train file types:')
-        for k, v in filetypes_train.items():
-            traces.log(f'    {v} {k}')
-        traces.log(f'')
-        traces.log(f'Test file types:')
-        for k, v in filetypes_test.items():
-            traces.log(f'    {v} {k}')
-        traces.log(f'')
+        _log_file_types_helper('Train', filetypes_train)
+        _log_file_types_helper('Test', filetypes_test)
         with open(os.path.join(traces.context().path, "source_files.json"), "w") as f:
             json.dump({
                 "run_id": run_id,
                 "train": filetypes_train,
                 "test": filetypes_test,
             }, f, indent=4)
 
+def _log_file_types_helper(dataset_type: str, filetypes: Dict[str, int]):
+    """Helper function to log file types for a specific dataset."""
+    traces.log(f'{dataset_type} file types:')
+    for k, v in filetypes.items():
+        traces.log(f'    {v} {k}')
+    traces.log('')
+
+def _log_config(user_cfg_nondefault: Dict[str, Any], cfg_builder: ConfigBuilder):
+    """Log the non-default configuration and LoRA settings."""
     if dist.get_rank() == 0:
         for k, v in user_cfg_nondefault.items():
             traces.log(f'Non-default parameter: {k:>20} {v}')
         with open(os.path.join(traces.context().path, "parameters_nondefault.json"), "w") as f:
             json.dump(user_cfg_nondefault, f, indent=4)
+        
         traces.log(f'Freeze exceptions: {cfg_builder.cfg["model_info"]["freeze_exceptions"]}')
         for k, v in cfg_builder.cfg["model_info"]["lora"].items():
             traces.log(f'Lora config: {k:>20} {v}')
         with open(os.path.join(traces.context().path, "config.json"), "w") as f:
             json.dump(cfg_builder.cfg, f, indent=4)
-        traces.log(f'Low memory mode: {user_cfg["low_gpu_mem_mode"]}')
-
-    assert cfg_builder.cfg['train_iters'] % cfg_builder.cfg['test_every'] == 0
-    assert cfg_builder.cfg['save_every'] % cfg_builder.cfg['test_every'] == 0
-
-    return cfg_builder.cfg
+        traces.log(f'Low memory mode: {cfg_builder.cfg["low_gpu_mem_mode"]}')
```

This refactored version of the `_build_finetune_config_by_heuristics` function includes the following improvements:

1. Added a docstring to explain the function's purpose and parameters.
2. Extracted helper functions to improve readability and maintainability:
   - `_process_user_config`
   - `_configure_with_heuristics`
   - `_configure_with_user_params`
   - `_set_common_config`
   - `_log_file_types`
   - `_log_file_types_helper`
   - `_log_config`
3. Improved the overall structure of the function, making it easier to understand the flow.
4. Ensured consistent formatting and naming conventions.
5. Removed commented-out code to improve clarity.
6. Added type hints to function parameters and return values.
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
