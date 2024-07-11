use std::path::PathBuf;

use itertools::Itertools;
use ropey::Rope;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::ast_interaction::parse_and_get_error_symbols;
use crate::at_tools::att_patch::tool::DefaultToolPatch;
use crate::call_validation::DiffChunk;
use crate::diffs::apply_diff_chunks_to_text;
use crate::files_in_workspace::read_file_from_disk;

pub struct WholeFileDiffFormat {}

impl WholeFileDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is make changes to provided files using the provided task.
To suggest changes to a file you MUST return the entire content of the updated file.
You MUST use this *file listing* format

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
   -- Review the tasks and files provided
   -- Identify the specific changes required
   -- Use chain of thoughts to make sure nothing will be missed
   -- Assess after diff is generated, including its format validity

2. **Generate files changes:**
-- To suggest changes to a file you MUST return the entire content of the updated file.

-- You MUST use this *file listing* format:
    path/to/filename.js
    {fence[0]}
    // entire file content ...
    // ... goes in between
    {fence[1]}

-- Every *file listing* MUST use this format:
--- First line: the filename with any originally provided path
--- Second line: opening {fence[0]}
--- ... entire content of the file ...
--- Final line: closing {fence[1]}

-- To suggest changes to a file you MUST return a *file listing* that contains the entire content of the file.

-- *NEVER* skip, omit or elide content from a *file listing* using "..." or by adding comments like "... rest of code..."!

-- Create a new file you MUST return a *file listing* which includes an appropriate filename, including any appropriate path.
"#.to_string()
    }

    pub async fn parse_message(
        _: &str,
    ) -> Result<Vec<DiffChunk>, String> {
        todo!()
    }
}


pub struct SearchReplaceDiffFormat {}

impl SearchReplaceDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is to create a diff in a specific format using the provided task and all given files.
Diff format is based on *SEARCH/REPLACE* blocks.

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
   -- Review the tasks and files provided
   -- Identify the specific changes required
   -- Use chain of thoughts to make sure nothing will be missed
   -- Assess after diff is generated, including its format validity

2. **Generate Diff:**
Every *SEARCH/REPLACE block* must use this format:
-- The opening fence and code language, eg: ```python
-- The start of search block: <<<<<<< SEARCH
-- A contiguous chunk of lines to search for in the existing source code
-- The dividing line: =======
-- The lines to replace into the source code
-- The end of the replace block: >>>>>>> REPLACE
-- The closing fence: ```
-- Every *SEARCH* section must *EXACTLY MATCH* the existing source code, character for character, including all comments, docstrings, formatting, etc.
-- *SEARCH/REPLACE* blocks will replace *all* matching occurrences.
-- Include enough lines to make the SEARCH blocks unique.
-- Include *ALL* the code being searched and replaced!
-- To move code, use 2 *SEARCH/REPLACE* blocks: 1 to delete it from its current location, 1 to insert it in the new location.
-- If you've opened *SEARCH/REPLACE block* you must close it.
-- ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!"#.to_string()
    }


    pub async fn parse_message(
        _: &str,
    ) -> Result<Vec<DiffChunk>, String> {
        todo!()
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
-- Return edits similar to unified diffs that `diff -U0` would produce.
-- Make sure you include the first 2 lines with the real file paths which were given before
-- Don't include timestamps with the file paths.
-- Start each hunk of changes with a `@@ ... @@` line.
-- Don't include line numbers like `diff -U0` does. The user's patch tool doesn't need them.
-- The user's patch tool needs CORRECT patches that apply cleanly against the current contents of the file!
-- Think carefully and make sure you include and mark all lines that need to be removed or changed as `-` lines!
-- Copy some code before `+`-only hunks to be able to find the place where to add.
-- Make sure you mark all new or modified lines with `+`.
-- Don't leave out any lines or the diff patch won't apply correctly.
-- Indentation matters in the diffs!
-- Start a new hunk for each section of the file that needs changes.
-- Only output hunks that specify changes with `+` or `-` lines.
-- Skip any hunks that are entirely unchanging ` ` lines.
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

            let mut hunk_line_idx: usize = 0;
            let mut has_active_chunk = false;
            let mut line_idx_start: usize = 0;
            let mut lines_to_add: String = String::new();
            let mut lines_to_remove: String = String::new();
            let mut line_idx: usize = 0;
            let mut line = String::new();
            loop {
                if line_idx >= file_lines.len() {
                    break;
                }
                line = file_lines[line_idx].clone();
                if hunk_line_idx >= edit.hunk.len() {
                    break;
                }
                let (edit_hunk_line, has_diff_sign, is_add_sign) = if edit.hunk[hunk_line_idx].starts_with("-") {
                    (edit.hunk[hunk_line_idx][1..].to_string(), true, false)
                } else if edit.hunk[hunk_line_idx].starts_with("+") {
                    (edit.hunk[hunk_line_idx][1..].to_string(), true, true)
                } else {
                    (edit.hunk[hunk_line_idx].to_string(), false, false)
                };

                if !is_add_sign && line == edit_hunk_line && has_diff_sign {
                    if !has_active_chunk {
                        line_idx_start = line_idx;
                    }
                    has_active_chunk = true;
                    lines_to_remove.push_str(&format!("{line}\n"));
                    hunk_line_idx += 1;
                    line_idx += 1;
                    continue;
                } else if is_add_sign && has_diff_sign {
                    if !has_active_chunk {
                        line_idx_start = line_idx;
                    }
                    has_active_chunk = true;
                    lines_to_add.push_str(&format!("{edit_hunk_line}\n"));
                    hunk_line_idx += 1;
                    continue;
                } else {
                    if has_active_chunk {
                        diff_chunks.push(DiffChunk {
                            file_name: filename.clone(),
                            file_action: "edit".to_string(),
                            line1: line_idx_start + 1,
                            line2: line_idx,
                            lines_remove: lines_to_remove.clone(),
                            lines_add: lines_to_add.clone(),
                        });
                        lines_to_remove.clear();
                        lines_to_add.clear();
                        has_active_chunk = false;
                    }
                    if line == edit_hunk_line {
                        hunk_line_idx += 1;
                    }
                    line_idx += 1;
                }
            }
            if has_active_chunk {
                while hunk_line_idx < edit.hunk.len() {
                    if edit.hunk[hunk_line_idx].starts_with("+") {
                        let edit_hunk_line = edit.hunk[hunk_line_idx][1..].to_string();
                        lines_to_add.push_str(&format!("{edit_hunk_line}\n"));
                        hunk_line_idx += 1;
                    } else {
                        break;
                    }
                }

                diff_chunks.push(DiffChunk {
                    file_name: filename.clone(),
                    file_action: "edit".to_string(),
                    line1: line_idx_start + 1,
                    line2: line_idx,
                    lines_remove: lines_to_remove.clone(),
                    lines_add: lines_to_add.clone(),
                });
            }

            if hunk_line_idx < edit.hunk.len() {
                return Err(format!(
                    "Couldn't parse a diff hunk from the {hunk_line_idx} line:\n```\n{}\n```",
                    &edit.hunk.iter().join("\n")
                ));
            }
        }

        Ok(diff_chunks)
    }
}

pub async fn parse_diff_chunks_from_message(
    ccx: &mut AtCommandsContext,
    message: &String,
) -> Result<String, String> {
    let chunks = match DefaultToolPatch::parse_message(message).await {
        Ok(chunks) => chunks,
        Err(err) => {
            return Err(format!("Error parsing diff: {:?}", err));
        }
    };

    if chunks.is_empty() {
        return Err("No diff chunks were found".to_string());
    }

    let gx = ccx.global_context.clone();
    let maybe_ast_module = gx.read().await.ast_module.clone();
    for chunk in chunks.iter() {
        let path = PathBuf::from(&chunk.file_name);
        let text_before = match read_file_from_disk(&path).await {
            Ok(text) => text,
            Err(err) => {
                let message = format!("Error reading file: {:?}, skipping ast assessment", err);
                return Err(message);
            }
        };
        let (text_after, _) = apply_diff_chunks_to_text(
            &text_before.to_string(),
            chunks.iter().enumerate().collect::<Vec<_>>(),
            vec![],
            1,
        );
        match &maybe_ast_module {
            Some(ast_module) => {
                let before_error_symbols = match parse_and_get_error_symbols(
                    ast_module.clone(),
                    &path,
                    &text_before,
                ).await {
                    Ok(symbols) => symbols,
                    Err(err) => {
                        warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                        continue;
                    }
                };
                let after_error_symbols = match parse_and_get_error_symbols(
                    ast_module.clone(),
                    &path,
                    &Rope::from_str(&text_after),
                ).await {
                    Ok(symbols) => symbols,
                    Err(err) => {
                        warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                        continue;
                    }
                };
                if before_error_symbols.len() < after_error_symbols.len() {
                    let message = format!(
                        "Error: the diff: {:?} introduced errors into the file: {:?}",
                        message,
                        path
                    );
                    return Err(message);
                }
            }
            None => {
                warn!("AST module is disabled, the diff assessment is skipping");
            }
        }
    }

    match serde_json::to_string_pretty(&chunks) {
        Ok(json_chunks) => Ok(json_chunks),
        Err(err) => Err(format!("Error diff chunks serializing: {:?}", err))
    }
}
