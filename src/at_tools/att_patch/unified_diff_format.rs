use std::path::PathBuf;

use itertools::Itertools;

use crate::call_validation::DiffChunk;
use crate::files_in_workspace::read_file_from_disk;

#[derive(Debug)]
struct Edit {
    path: Option<String>,
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

fn search_text_location(hunk: &[String], file_lines: &[String]) -> Option<(usize, usize)> {
    let mut minus_blocks: usize = 0;
    let mut initial_text_lines = hunk
        .iter()
        .take_while(|x| !x.starts_with("+"))
        .map(|x| {
            if x.starts_with("-") {
                minus_blocks += 1;
            };
            if x.is_empty() { x.to_string() } else { x[1..].to_string() }
        })
        .collect::<Vec<_>>();
    if initial_text_lines.is_empty() {
        return Some((0, 1))  // it's only left to put data before the first file row
    }
    for i in 0..=file_lines.len() - initial_text_lines.len() {
        if file_lines[i..i + initial_text_lines.len()] == initial_text_lines[..] {
            let hunk_offset = initial_text_lines.len() - minus_blocks;
            return Some((hunk_offset, i + hunk_offset));
        }
    }

    // make another attempt using only content from `-` blocks
    if minus_blocks > 0 {
        let minus_text_lines = initial_text_lines
            .iter()
            .cloned()
            .rev()
            .take(minus_blocks)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        for i in 0..=file_lines.len() - minus_text_lines.len() {
            if file_lines[i..i + minus_text_lines.len()] == minus_text_lines[..] {
                let hunk_offset = initial_text_lines.len() - minus_blocks;
                return Some((hunk_offset, i + minus_text_lines.len() - minus_blocks));
            }
        }
    }
    
    None
}

fn parse_diff_chunk(hunk: &[String], file_lines: &[String]) -> Result<(usize, usize, DiffChunk), String> {
    fn strip_hunk(s: &String) -> String {
        if s.is_empty() {
            s.to_string()
        } else {
            s[1..].to_string()
        }
    }
    
    let mut hunk_line_idx: usize = 0;
    let mut file_line_idx: usize = 0;
    let mut lines_to_add: String = String::new();
    let mut lines_to_remove: String = String::new();
    let mut has_started_with_minus = false;
    loop {
        if hunk_line_idx >= hunk.len() {
            let chunk = DiffChunk {
                file_name: "".to_string(),
                file_action: "edit".to_string(),
                line1: if has_started_with_minus { 1 } else { 0 },
                line2: if has_started_with_minus { 1 + file_line_idx } else { file_line_idx },
                lines_remove: lines_to_remove.clone(),
                lines_add: lines_to_add.clone(),
            };
            return Ok((hunk_line_idx, file_line_idx, chunk));
        }
        if file_line_idx >= file_lines.len() {
            return Err("File has no more lines to parse while the duff hunk still has unparsed data".to_string());
        }

        let file_line = &file_lines[file_line_idx];
        let (hunk_line, has_diff_sign, is_add_sign) = if hunk[hunk_line_idx].starts_with("-") {
            if hunk_line_idx == 0 {
                has_started_with_minus = true;
            }
            (strip_hunk(&hunk[hunk_line_idx]), true, false)
        } else if hunk[hunk_line_idx].starts_with("+") {
            (strip_hunk(&hunk[hunk_line_idx]), true, true)
        } else {
            (strip_hunk(&hunk[hunk_line_idx]), false, false)
        };

        if has_diff_sign {
            if !is_add_sign && *file_line == hunk_line {
                lines_to_remove.push_str(&format!("{file_line}\n"));
                file_line_idx += 1;
                hunk_line_idx += 1;
                continue;
            } else {
                lines_to_add.push_str(&format!("{hunk_line}\n"));
                hunk_line_idx += 1;
                continue;
            }
        } else {
            let chunk = DiffChunk {
                file_name: "".to_string(),
                file_action: "edit".to_string(),
                line1: if has_started_with_minus { 1 } else { 0 },
                line2: if has_started_with_minus { 1 + file_line_idx } else { file_line_idx },
                lines_remove: lines_to_remove.clone(),
                lines_add: lines_to_add.clone(),
            };
            return Ok((hunk_line_idx, file_line_idx, chunk));
        }
    }
}

pub struct UnifiedDiffFormat {}

impl UnifiedDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is to create a unified diff format output based on the provided task and all files.

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
-- Review the tasks and files provided
-- Identify the specific changes required
-- Use chain of thoughts to make sure nothing will be missed
-- Explain every change in all files before making the diff and for each explanation write if you should use `-` or `+`
-- Assess after diff is generated, including its format validity (`+` and `-` symbols are in the right places and nothing is missing)!

2. **Generate Diff:**
-- Don't forget to make changes to all given files
-- Return edits similar to unified diffs that `diff -U2` would produce.
-- Make sure you include the first 2 lines with the real file paths which were given before
-- Don't include timestamps with the file paths.
-- Start each hunk of changes with a `@@ ... @@` line.
-- Don't include line numbers like `diff -U2` does. The user's patch tool doesn't need them.
-- The user's patch tool needs CORRECT patches that apply cleanly against the current contents of the file!
-- Think carefully and make sure you include and mark all lines that need to be removed or changed as `-` lines!
-- Copy some code before `+`-only hunks to be able to find the place where to add, add empty space character ` ` for copied text including empty lines.
-- Make sure you mark all new or modified lines with `+`.
-- Don't leave out any lines or the diff patch won't apply correctly.
-- Indentation matters in the diffs!
-- Start a new hunk for each section of the file that needs changes.
-- Only output hunks that specify changes with `+` or `-` lines.
-- Output hunks in whatever order makes the most sense.
-- Hunks don't need to be in any particular order.
-- When editing a function, method, loop, etc. use a hunk to replace the *entire* code block.
-- Delete the entire existing version with `-` lines and then add a new, updated version with `+` lines. This will help you generate correct code and correct diffs.
-- To move code within a file, use 2 hunks: 1 to delete it from its current location, 1 to insert it in the new location

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
-    for i in range(2, int(math.sqrt(x)) + 1):
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
 def test_todo():
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

        let mut diff_chunks: Vec<DiffChunk> = vec![];
        for edit in edits.iter() {
            let filename = match edit.path.clone() {
                Some(p) => p,
                None => {
                    return Err(format!("Cannot get a file name from the diff chunk, skipping it: {edit:?}"));
                }
            };
            let file_lines = read_file_from_disk(&PathBuf::from(filename.clone()))
                .await
                .map_err(|e| {
                    format!("couldn't read file: {:?}. Error: {}", filename, e)
                })
                .map(|x| x.lines().into_iter().map(|x| x.to_string().trim_end().to_string()).collect::<Vec<_>>())?;


            let mut hunk_line_cursor: usize = 0;
            let mut file_line_cursor: usize = 0;
            loop {
                if hunk_line_cursor >= edit.hunk.len() {
                    break;
                };
                if file_line_cursor >= file_lines.len() {
                    return Err("File has no more lines to parse while the duff hunk still has unparsed data".to_string());
                };

                let (hunk_line_cursor_offset, new_file_line_cursor_offset) = match search_text_location(
                    &edit.hunk[hunk_line_cursor..], &file_lines[file_line_cursor..]
                ) {
                    Some(res) => res,
                    None => {
                        return Err(format!("Couldn't find the text location in the file: {}", filename));
                    }
                };
                hunk_line_cursor += hunk_line_cursor_offset;
                file_line_cursor += new_file_line_cursor_offset;

                let (hunk_line_cursor_offset, new_file_line_cursor_offset, mut diff_chunk) = match parse_diff_chunk(
                    &edit.hunk[hunk_line_cursor..], &file_lines[file_line_cursor..]
                ) {
                    Ok(res) => res,
                    Err(err) => {
                        return Err(
                            format!("Couldn't parse a diff hunk from the {hunk_line_cursor} line, {err}:\n```\n{}\n```",
                                    &edit.hunk.iter().join("\n"))
                        );
                    }
                };
                diff_chunk.file_name = filename.clone();
                diff_chunk.line1 += file_line_cursor;
                diff_chunk.line2 += file_line_cursor;
                hunk_line_cursor += hunk_line_cursor_offset;
                file_line_cursor += new_file_line_cursor_offset;
                if diff_chunk.is_empty() {
                    continue
                }
                diff_chunks.push(diff_chunk);
            };
        }

        Ok(diff_chunks)
    }
}

#[cfg(test)]
mod tests {
    use log::info;
    use crate::at_tools::att_patch::unified_diff_format::UnifiedDiffFormat;
    use crate::call_validation::DiffChunk;

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
        assert_eq!(result, gt_result);
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
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 5,
                line2: 5,
                lines_remove: "".to_string(),
                lines_add: "    # Frog class description\n".to_string(),
            }
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
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
        assert_eq!(result, gt_result);
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
        assert_eq!(result, gt_result);
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
        assert_eq!(result, gt_result);
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
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/frog.py".to_string(),
                file_action: "edit".to_string(),
                line1: 27,
                line2: 27,
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
        assert_eq!(result, gt_result);
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
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/set_as_avatar.py".to_string(),
                file_action: "edit".to_string(),
                line1: 24,
                line2: 24,
                lines_remove: "".to_string(),
                lines_add: "        # extra row 1\n        # extra row 2\n        # extra row 3\n".to_string(),
            },
        ];
        let result = UnifiedDiffFormat::parse_message(input).await.expect(
            "Failed to parse diff message"
        );
        assert_eq!(result, gt_result);
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
        assert_eq!(result, gt_result);
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
        let gt_result = vec![
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 12,
                line2: 12,
                lines_remove: "".to_string(),
                lines_add: "    frog3 = Frog()\n".to_string(),
            },
            DiffChunk {
                file_name: "tests/emergency_frog_situation/holiday.py".to_string(),
                file_action: "edit".to_string(),
                line1: 14,
                line2: 14,
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
        assert_eq!(result, gt_result);
    }
}
