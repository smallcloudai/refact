use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use async_trait::async_trait;
use itertools::Itertools;
use rand::distributions::Alphanumeric;
use rand::Rng;
use ropey::Rope;
use serde_json::Value;
use tracing::{info, warn};

use crate::ast::ast_index::RequestSymbolType;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::execute_at_file;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ChatPost, ContextEnum, DiffChunk, SamplingParameters};
use crate::diffs::apply_diff_chunks_to_text;
use crate::files_in_workspace::{Document, read_file_from_disk};
use crate::scratchpads;

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
-- Make sure you include the first 2 lines with the file paths.
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
--- mathweb/flask/app.py
+++ mathweb/flask/app.py
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


const DEFAULT_MODEL_NAME: &str = "gpt-4o";
const MAX_TOKENS: usize = 32000;
const TEMPERATURE: f32 = 0.1;
pub type DefaultToolPatch = UnifiedDiffFormat;


struct PatchArguments {
    paths: Vec<String>,
    symbol_names: Option<Vec<String>>,
    todo: String,
}

async fn symbols_to_signatures_context(symbols: &Vec<SymbolInformation>) -> String {
    let mut context: String = "".to_string();
    for s in symbols.iter() {
        let decl_sign = match s.get_declaration_content_from_file().await {
            Ok(sign) => sign,
            Err(err) => {
                warn!("Cannot get a content for symbol {:?}: {err}", s.name);
                continue;
            }
        };
        context.push_str(&format!("```\n{decl_sign}\n```\n"))
    }
    context
}

async fn parse_initial_arguments(
    args: &HashMap<String, Value>,
    ccx: &mut AtCommandsContext,
) -> Result<PatchArguments, String> {
    let paths = match args.get("paths") {
        Some(Value::String(s)) => s.split(",").map(|x| x.to_string()).collect::<Vec<String>>(),
        Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) }
        None => { return Err("argument `path` is not a string".to_string()) }
    };
    let mut corrected_paths = vec![];
    for p in paths.into_iter() {
        let corrected = crate::files_correction::correct_to_nearest_filename(
            ccx.global_context.clone(),
            &p,
            false,
            1,
        ).await;
        if corrected.is_empty() {
            return Err(format!("Cannot find a file {p}"));
        }
        corrected_paths.push(corrected[0].clone());
    }
    let symbol_names = match args.get("symbols") {
        Some(Value::String(s)) => Some(s.split(",").map(|x| x.to_string()).collect::<Vec<String>>()),
        Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) }
        None => None
    };
    let todo = match args.get("todo") {
        Some(Value::String(s)) => s.clone(),
        Some(v) => { return Err(format!("argument `todo` is not a string: {:?}", v)) }
        None => { "".to_string() }
    };
    Ok(PatchArguments {
        paths: corrected_paths,
        symbol_names,
        todo,
    })
}

async fn make_prompt(
    args: &PatchArguments,
    ccx: &mut AtCommandsContext,
) -> Result<(String, Option<String>), String> {
    let maybe_symbols = if let Some(ast_module) = ccx.global_context.read().await.ast_module.clone() {
        let mut symbols = vec![];
        if let Some(symbols_names) = args.symbol_names.clone() {
            for name in symbols_names.into_iter() {
                let res = match ast_module
                    .read()
                    .await
                    .search_by_name(name, RequestSymbolType::Declaration, false, 1)
                    .await {
                    Ok(s) => s.search_results
                        .get(0)
                        .map(|x| x.symbol_declaration.clone()),
                    Err(_) => None
                };
                if let Some(s) = res {
                    symbols.push(s.clone());
                }
            }
        } else {
            for filename in args.paths.iter() {
                if let Ok(path) = PathBuf::from_str(filename) {
                    let doc = Document::new(&path);
                    match ast_module
                        .read()
                        .await
                        .decl_symbols_from_imports_by_file_path(&doc, 1)
                        .await {
                        Ok(s) => {
                            s.search_results
                                .iter()
                                .map(|x| {
                                    symbols.push(x.symbol_declaration.clone());
                                    s.clone()
                                })
                                .collect::<Vec<_>>()
                        }
                        Err(err) => {
                            warn!("Cannot import symbols for path {:?}: {err}", path);
                            continue;
                        }
                    };
                } else {
                    warn!("Cannot parse path: {filename}");
                    continue;
                }
            }
        }
        Some(symbols)
    } else {
        None
    };

    let extra_context = if let Some(symbols) = maybe_symbols {
        Some(symbols_to_signatures_context(&symbols).await)
    } else {
        None
    };
    Ok((DefaultToolPatch::prompt(), extra_context))
}

async fn run_chat(
    args: &PatchArguments,
    ccx: &mut AtCommandsContext,
    system_prompt: String,
    maybe_extra_context: Option<String>,
) -> Result<String, String> {
    let gx = ccx.global_context.clone();
    let mut chat_messages = vec![
        ChatMessage::new(
            "system".to_string(),
            system_prompt.to_string(),
        )
    ];
    for file in args.paths.iter() {
        match execute_at_file(ccx, file.clone()).await {
            Ok(res) => {
                chat_messages.push(ChatMessage::new(
                    "user".to_string(),
                    format!("{}\n```\n{}```\n\n", res.file_name, res.file_content).to_string(),
                ));
            }
            Err(err) => {
                warn!("Cannot find a `{file}`: {err}");
            }
        }
    }
    if let Some(extra_context) = maybe_extra_context {
        chat_messages.push(ChatMessage::new(
            "user".to_string(),
            format!("Extra context for the files:\n{}", extra_context).to_string(),
        ));
    }
    chat_messages.push(ChatMessage::new(
        "user".to_string(),
        format!("The task:\n{}", args.todo).to_string(),
    ));

    let mut chat_post = ChatPost {
        messages: chat_messages,
        parameters: SamplingParameters {
            max_new_tokens: 4096,
            temperature: Some(TEMPERATURE),
            top_p: None,
            stop: vec![],
        },
        model: DEFAULT_MODEL_NAME.to_string(),
        scratchpad: "".to_string(),
        stream: Some(false),
        temperature: Some(TEMPERATURE),
        max_tokens: MAX_TOKENS,
        tools: None,
        only_deterministic_messages: false,
        chat_id: "".to_string(),
    };
    let caps = crate::global_context::try_load_caps_quickly_if_not_present(
        gx.clone(), 0,
    )
        .await
        .map_err(|e| {
            warn!("No caps: {:?}", e);
            "Network error communicating with the model (1)".to_string()
        })?;

    let (model_name, scratchpad_name, scratchpad_patch, n_ctx, _) = crate::http::routers::v1::chat::lookup_chat_scratchpad(
        caps.clone(),
        &chat_post,
    ).await?;
    let (client, api_key) = {
        let cx_locked = gx.write().await;
        (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone())
    };
    let mut scratchpad = scratchpads::create_chat_scratchpad(
        gx.clone(),
        caps,
        model_name.clone(),
        &chat_post.clone(),
        &scratchpad_name,
        &scratchpad_patch,
        false,
        false,
    ).await?;
    let prompt = scratchpad.prompt(
        n_ctx,
        &mut chat_post.parameters,
    ).await?;

    let t1 = std::time::Instant::now();
    let messages = crate::restream::scratchpad_interaction_not_stream_json(
        gx.clone(),
        scratchpad,
        "chat".to_string(),
        &prompt,
        model_name,
        client,
        api_key,
        &chat_post.parameters,
        chat_post.only_deterministic_messages,
    ).await.map_err(|e| {
        warn!("Network error communicating with the (2): {:?}", e);
        "Network error communicating with the model (2)".to_string()
    })?;
    info!("patch generation took {:?}ms", t1.elapsed().as_millis() as i32);

    let choices_array = match messages["choices"].as_array() {
        Some(array) => array,
        None => return Err("Unable to get choices array from JSON".to_string()),
    };

    let choice0 = match choices_array.get(0) {
        Some(Value::Object(o)) => o,
        Some(v) => { return Err(format!("choice[0] is not a dict: {:?}", v)) }
        None => { return Err("choice[0] doesn't exist".to_string()) }
    };

    let choice0_message = match choice0.get("message") {
        Some(Value::Object(o)) => o,
        Some(v) => { return Err(format!("choice[0].message is not a dict: {:?}", v)) }
        None => { return Err("choice[0].message doesn't exist".to_string()) }
    };

    match choice0_message.get("content") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(v) => { return Err(format!("choice[0].message.content is not a string: {:?}", v)) }
        None => { return Err("choice[0].message.content doesn't exist".to_string()) }
    }
}


async fn parse_diff_from_message(
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
                let dummy_filename = PathBuf::from(rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect::<String>());
                let new_filename = dummy_filename.with_extension(
                    path.extension().unwrap_or_default()
                );
                let before_doc = Document { path: new_filename.clone(), text: Some(text_before.clone()) };
                let after_doc = Document { path: new_filename, text: Some(Rope::from_str(&text_after)) };

                let before_error_symbols = match ast_module.read()
                    .await
                    .file_markup(&before_doc)
                    .await {
                    Ok(symbols) => symbols
                        .symbols_sorted_by_path_len
                        .into_iter()
                        .filter(|x| x.is_error)
                        .collect::<Vec<_>>(),
                    Err(err) => {
                        warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                        continue;
                    }
                };
                let after_error_symbols = match ast_module.read()
                    .await
                    .file_markup(&after_doc)
                    .await {
                    Ok(symbols) => symbols
                        .symbols_sorted_by_path_len
                        .into_iter()
                        .filter(|x| x.is_error)
                        .collect::<Vec<_>>(),
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

pub struct ToolPatch {}
#[async_trait]
impl Tool for ToolPatch {
    async fn execute(
        &self,
        ccx: &mut AtCommandsContext,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let args = match parse_initial_arguments(args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        let (system_prompt, maybe_extra_context) = match make_prompt(&args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        let answer = match run_chat(&args, ccx, system_prompt, maybe_extra_context).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        info!("Tool patch answer: {answer}");
        match parse_diff_from_message(ccx, &answer).await {
            Ok(res) => {
                info!("Tool patch diff: {:?}", res);
                Ok(vec![(ContextEnum::ChatMessage(ChatMessage {
                    role: "diff".to_string(),
                    content: res,
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                }))])
            }
            Err(err) => {
                warn!(err);
                Ok(vec![ContextEnum::ChatMessage(ChatMessage {
                    role: "diff".to_string(),
                    content: format!("Can't make any changes: {err}"),
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                })])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_diff_message() {
        let input = r#"Let's break down the changes needed to implement the task:

1. **Track the last bounding box position for each track:**
   - Add a new dictionary `_last_bboxes` to store the last bounding box positions.
   - Initialize this dictionary in the constructor.

2. **Check for duplicates before emitting an event:**
   - Add a method `_is_significant_movement` to check if the current bounding box has moved significantly compared to the last recorded position.

3. **Emit unique events only if the bounding box has moved significantly:**
   - Update the `_handle_abandoned_item` method to use `_is_significant_movement` before emitting an event.
   - Update the last bounding box position after emitting an event.

Here is the unified diff format for the changes:

```diff
--- /home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py
+++ /home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py
@@ ... @@
    def __init__(self, circle_radius: float, abandonment_frames: int, max_age: int, moved_frames: int,  **kwargs):
        ProcessConsumerProducer.__init__(self, **kwargs)
        self._circle_radius = circle_radius
        self._abandonment_frames = abandonment_frames
        self._max_age = max_age
        self._moved_frames = moved_frames
        self._abandonment_items = {}
+        self._last_bboxes = {}  # Track last bounding box positions

@@ ... @@
    def _handle_abandoned_item(self, item: Prediction, message: Message) -> None:
        if item.track_idx not in self._abandonment_items:
            self._abandonment_items[item.track_idx] = AbandonmentItem(prediction=item)
        else:
            self._abandonment_items[item.track_idx].moved_count = 0
            frames_diff = item.frame_idx - self._abandonment_items[item.track_idx].prediction.frame_idx
            if frames_diff > self._abandonment_frames and not self._abandonment_items[item.track_idx].is_sent:
+                if self._is_significant_movement(item):
                    event = Event(prediction=self._abandonment_items[item.track_idx].prediction, event_type=EventType.DIST_2)
                    message.data.events.append(event)
                    self._abandonment_items[item.track_idx].is_sent = True
+                    self._last_bboxes[item.track_idx] = item.bbox.center_xywh()[:2]  # Update last bbox position

+    def _is_significant_movement(self, item: Prediction) -> bool:
+        if item.track_idx not in self._last_bboxes:
+            return True
+        last_x, last_y = self._last_bboxes[item.track_idx]
+        current_x, current_y = item.bbox.center_xywh()[:2]
+        distance = math.sqrt((last_x - current_x) ** 2 + (last_y - current_y) ** 2)
+        return distance > self._circle_radius / 2  # Consider significant if moved more than half the radius
```

Explanation of changes:
1. **Constructor Update:**
   - Added `self._last_bboxes = {}` to track the last bounding box positions. (`+`)

2. **_handle_abandoned_item Method Update:**
   - Added a check using `_is_significant_movement(item)` before emitting an event. (`+`)
   - Updated the last bounding box position after emitting an event. (`+`)

3. **New Method _is_significant_movement:**
   - Added a new method `_is_significant_movement` to check if the current bounding box has moved significantly compared to the last recorded position. (`+`)

This diff should be applied to the file `/home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py`. The changes ensure that duplicate events are filtered out based on their relative position using the last bounding box in the track."#;
        let result = DefaultToolPatch::parse_message(input).await.unwrap();
        info!("result: {:?}", result);
    }
}
