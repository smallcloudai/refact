use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files, AstIndexService};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{
    ChatContent, ChatMessage, CodeCompletionPost, CursorPosition, SamplingParameters,
};
use crate::caps::resolve_completion_model;
use crate::completion_cache;
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot, ScratchpadAbstract};
use crate::scratchpads::comments_parser::parse_comments;
use crate::telemetry::snippets_collection;
use crate::telemetry::telemetry_structs;
use async_trait::async_trait;
use ropey::Rope;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Instant;
use std::vec;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::{info, warn};
use crate::ast::ast_db::doc_defs;
use crate::ast::ast_structs::AstDefinition;
use crate::scratchpads::completon_rag::retrieve_ast_based_extra_context;

const DEBUG: bool = false;
const SYSTEM_PROMPT: &str = r#"You are given a code file, <BLOCK_OF_CODE> from that file and an extra context from other files.
An unfinished line in the <BLOCK_OF_CODE> is marked with the <CURSOR>.
Your task is to complete the code after the <CURSOR> by rewriting the <BLOCK_OF_CODE> using the provided context and make the <REWRITTEN_BLOCK_OF_CODE>.
Ensure the <REWRITTEN_BLOCK_OF_CODE> introduces all necessary updates to the <BLOCK_OF_CODE> such as code completion, function definitions, or comments.
Keep identation symbols unchanged. Do not output multiple <REWRITTEN_BLOCK_OF_CODE> blocks and make sure changes are made only after the <CURSOR>"#;
const SYSTEM_PROMPT_USERS_INTENTION: &str = r#"You are given a code file, <BLOCK_OF_CODE> from that file, an extra context from other files, and a user's intention.
Rewrite the <BLOCK_OF_CODE> to fulfill the user's intention, starting from the <CURSOR> position using the provided context and make the <REWRITTEN_BLOCK_OF_CODE>.
Ensure the <REWRITTEN_BLOCK_OF_CODE> introduces all necessary updates to the <BLOCK_OF_CODE> such as code completion, function definitions, or comments.
Keep identation symbols unchanged. Do not output multiple <REWRITTEN_BLOCK_OF_CODE> blocks and make sure changes are made only after the <CURSOR>.
Strictly follow the user's intention.
User's intention:
<comment>"#;
const MAX_ROWS_UP_OR_DOWNS: usize = 10;
const MIN_ROWS_TO_SKIP_CARET: usize = 2;
const SUBBLOCK_REQUIRED_TOKENS: usize = 128;
const CURSORFILE_MIN_TOKENS: usize = 128;
const MAX_NEW_TOKENS: usize = 1024;  // it's quite high since we want to avoid having a stripped message
const TEMPERATURE_INITIAL: f32 = 0.0;
const TEMPERATURE_NOCACHE: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct SubBlock {
    before_lines: Vec<String>,
    cursor_line: String,
    after_lines: Vec<String>,
    after_lines_extra: Vec<String>,
}

impl SubBlock {
    fn prompt(&self) -> Result<String, String> {
        let mut code = self
            .before_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("");

        code.push_str(format!("{}<CURSOR>\n", self.cursor_line.trim_end().to_string()).as_str());
        code.push_str(
            self.after_lines
                .iter()
                .map(|x| x.replace("\r\n", "\n"))
                .collect::<Vec<_>>()
                .join("")
                .as_str(),
        );
        Ok(format!("<BLOCK_OF_CODE>:\n```\n{code}\n```"))
    }

    fn before_lines_str(&self) -> String {
        self.before_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("")
    }

    fn after_lines_str(&self) -> String {
        self.after_lines_extra
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("")
    }
}

fn prepare_cursor_file(
    tokenizer: &HasTokenizerAndEot,
    max_tokens: usize,
    file_name: &PathBuf,
    file_text: &Rope,
    cursor_pos: &CursorPosition,
) -> Result<(String, usize, (usize, usize)), String> {
    let mut output_lines: VecDeque<String> = VecDeque::new();
    let mut tokens_used: usize = 0;
    let mut line_idx_offset: i32 = 1;

    let line = file_text.line(cursor_pos.line as usize).to_string();
    output_lines.push_front(line.to_string());
    tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
    if tokens_used > max_tokens {
        return Err("Tokens limit is too small to fit the cursor file".to_string());
    }
    let mut line1: usize = usize::MIN;
    let mut line2: usize = usize::MIN;
    loop {
        if cursor_pos.line - line_idx_offset >= 0 {
            let line = file_text.line((cursor_pos.line - line_idx_offset) as usize).to_string();
            tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
            if tokens_used > max_tokens {
                break;
            }
            output_lines.push_front(line);
            line1 = (cursor_pos.line - line_idx_offset) as usize;
        }
        if cursor_pos.line + line_idx_offset < file_text.len_lines() as i32 {
            let line = file_text.line((cursor_pos.line + line_idx_offset) as usize).to_string();
            tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
            if tokens_used > max_tokens {
                break;
            }
            output_lines.push_back(line);
            line2 = (cursor_pos.line + line_idx_offset) as usize;
        }

        if cursor_pos.line - line_idx_offset < 0
            && cursor_pos.line + line_idx_offset >= file_text.len_lines() as i32
        {
            break;
        }

        line_idx_offset += 1;
    }
    let file_text = output_lines
        .into_iter()
        .map(|x| x.replace("\r\n", "\n"))
        .collect::<Vec<_>>()
        .join("");
    let data = format!(
        "File name:\n{}\nContent:\n```\n{file_text}\n```",
        file_name.to_string_lossy()
    );
    let tokens_used = tokenizer.count_tokens(&data).unwrap_or(0) as usize;
    Ok((data, tokens_used, (line1, line2)))
}

pub async fn get_cursor_symbol_from_doc(
    ast_service: Option<Arc<AMutex<AstIndexService>>>,
    cpath: &PathBuf,
    cursor_pos: &CursorPosition,
) -> Option<Arc<AstDefinition>> {
    let ast_service = ast_service?;
    let ast_index = ast_service.lock().await.ast_index.clone();
    let cpath_str = cpath.to_string_lossy().to_string();
    ast_indexer_enqueue_files(ast_service.clone(), &vec![cpath_str.clone()], true).await;
    ast_indexer_block_until_finished(ast_service.clone(), 20, true).await;
    let doc_syms = doc_defs(ast_index, &cpath_str).await;
    doc_syms
        .iter()
        .filter(
            |s| cursor_pos.line >= s.full_line1().saturating_sub(1) as i32 && cursor_pos.line <= s.full_line2() as i32
        )
        .cloned()
        .min_by_key(|x| x.full_line2().saturating_sub(x.full_line1()))
}

async fn prepare_subblock(
    ast_service: Option<Arc<AMutex<AstIndexService>>>,
    tokenizer: &HasTokenizerAndEot,
    max_tokens: usize,
    cpath: &PathBuf,
    file_text: &Rope,
    cursor_pos: &CursorPosition,
    max_rows_up_or_downs: usize,
    min_rows_to_skip_caret: usize,
) -> Result<(SubBlock, usize), String> {
    let mut subblock: SubBlock = SubBlock {
        before_lines: vec![],
        cursor_line: String::new(),
        after_lines: vec![],
        after_lines_extra: vec![]
    };
    let mut tokens_used: usize = 0;

    let line = file_text.line(cursor_pos.line as usize).to_string();
    subblock.cursor_line = line.to_string();
    tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
    if tokens_used > max_tokens {
        return Err("Tokens limit is too small to fit the code subblock".to_string());
    }

    if let Some(symbol) = get_cursor_symbol_from_doc(ast_service.clone(), cpath, cursor_pos).await {
        let min_rows_to_include = 2;
        for idx in symbol.full_line1().saturating_sub(1)..symbol.full_line2() + 1 {
            if idx < file_text.len_lines() {
                let line = file_text.line(idx).to_string();
                tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
                if idx < cursor_pos.line as usize {
                    subblock.before_lines.push(line);
                } else if idx > cursor_pos.line as usize {
                    subblock.after_lines_extra.push(line.clone());
                    if tokens_used <= max_tokens || subblock.after_lines.len() < min_rows_to_include {
                        subblock.after_lines.push(line);
                    }
                }
            }
        }
    } else {
        for (c, i) in (cursor_pos.line - max_rows_up_or_downs as i32..cursor_pos.line).rev().enumerate() {
            if i >= 0 {
                let line = file_text.line(i as usize).to_string();
                if c >= min_rows_to_skip_caret && line.trim().is_empty() {
                    break;
                }
                tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
                subblock.before_lines.insert(0, line);
                if tokens_used > max_tokens {
                    return Err(
                        "Tokens limit is too small to fit the context for the code subblock"
                            .to_string(),
                    );
                }
            }
        }
        for (c, i) in (cursor_pos.line + 1..cursor_pos.line + max_rows_up_or_downs as i32).enumerate() {
            if i < file_text.len_lines() as i32 {
                let line = file_text.line(i as usize).to_string();
                if c >= min_rows_to_skip_caret && line.trim().is_empty() {
                    break;
                }
                tokens_used += tokenizer.count_tokens(&line).unwrap_or(0) as usize;
                if tokens_used > max_tokens {
                    break;
                }
                subblock.after_lines.push(line);
            }
        }

        for i in cursor_pos.line + 1..cursor_pos.line + max_rows_up_or_downs as i32 {
            if i < file_text.len_lines() as i32 {
                subblock.after_lines_extra.push(file_text.line(i as usize).to_string());
            }
        }
    }
    Ok((subblock, tokens_used))
}

fn skip_similar_letters(a: &str, b: &str) -> String {
    let mut found_idx = None;
    for (idx, (ch_a, ch_b)) in a.chars().zip(b.chars()).enumerate() {
        if ch_a != ch_b {
            found_idx = Some(idx);
            break;
        }
    }
    if let Some(idx) = found_idx {
        b.split_at(idx).1.to_string()
    } else {
        if b.len() >= a.len() {
            b.split_at(a.len()).1.to_string()
        } else {
            "".to_string()
        }
    }
}

fn skip_similar_rows(pred_text: &Vec<String>, text_to_remove: &Vec<String>) -> Vec<String> {
    fn is_too_simple_to_compare(s: &String) -> bool {
        if s.trim().is_empty() {
            return true;
        }
        let simple_tokens = vec![
            ")", "(", "{", "}", "[", "]",  // Parentheses, braces, brackets
            "+", "-", "*", "/", "%",       // Arithmetic operators
            "=", "==", "!=", ">", "<",     // Comparison operators
            "&&", "||", "!",               // Logical operators
            ",", ".", ";", ":", "?",       // Punctuation
            "|", "&", "^", "~", ">>", "<<" // Bitwise operators
        ];
        simple_tokens.contains(&s.as_str())
    }

    let mut pred_text_trimmed = pred_text.clone();
    for to_remove_row in text_to_remove.iter() {
        if pred_text_trimmed.is_empty() {
            return pred_text_trimmed;
        }
        // if is_too_simple_to_compare(to_remove_row) {
        //     continue
        // }

        for idx in 0..(if to_remove_row.trim().is_empty() {1} else {pred_text_trimmed.len()}) {
            if *to_remove_row == pred_text_trimmed[idx] {
                pred_text_trimmed = pred_text_trimmed[idx + 1..].to_vec();
                break;
            }
            if !is_too_simple_to_compare(&to_remove_row)
                && !to_remove_row.trim().is_empty()
                && to_remove_row.trim_start() == pred_text_trimmed[idx].trim_start() {
                pred_text_trimmed = pred_text_trimmed[idx + 1..].to_vec();
                break;
            }
        }
    }
    pred_text_trimmed
}

fn retrieve_a_comment(source: &String, cpath: &PathBuf, cursor: &CursorPosition) -> Option<String> {
    let mut has_a_comment_right_after_the_cursor: bool = false;
    let comments = parse_comments(
        &source,
        &cpath
            .extension()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or("".to_string()),
    );
    let initial_comment = comments
        .iter()
        .map(|x| {
            has_a_comment_right_after_the_cursor |=
                x.start_line == (cursor.line + 1) as usize && !x.is_inline;
            x
        })
        .filter(|x| x.end_line == cursor.line as usize && !x.is_inline)
        .cloned()
        .collect::<Vec<_>>();
    if !has_a_comment_right_after_the_cursor {
        if let Some(c) = initial_comment.get(0) {
            let mut comments_to_combine = vec![c];
            for idx in (0..c.end_line - 1).rev() {
                if let Some(found_c) = comments
                    .iter()
                    .find(|x| x.end_line == idx as usize && !x.is_inline)
                {
                    comments_to_combine.push(found_c);
                } else {
                    break;
                }
            }
            let mut combined_text: String = "".to_string();
            for c in comments_to_combine.iter().rev() {
                combined_text += format!("{}", c.text).as_str();
            }
            Some(combined_text)
        } else {
            None
        }
    } else {
        None
    }
}

fn unfence_the_last_code_block(text: &String) -> Option<String> {
    let mut blocks: Vec<String> = vec![];
    let mut current_block: Option<String> = None;
    for line in Rope::from_str(text).lines() {
        if line.to_string().starts_with("```") {
            if let Some(block) = current_block {
                blocks.push(block);
                current_block = None;
            } else {
                current_block = Some(String::new());
            }
        } else {
            if let Some(block) = &mut current_block {
                block.push_str(&line.to_string());
            }
        }
    }
    // if there is a block without a closing ```
    if let Some(block) = current_block {
        blocks.push(block);
    }

    blocks.iter().last().cloned()
}

fn process_n_choices(
    subblock: &mut Option<SubBlock>,
    choices: &Vec<String>,
    finish_reasons: &Vec<FinishReason>,
    is_multiline: bool,
    data4cache: &mut completion_cache::CompletionSaveToCache,
) -> Vec<Value> {
    let subblock_ref = subblock
        .as_mut()
        .expect("cursor_subblock must be initialized in the prompt");
    let after_lines_str = subblock_ref.after_lines_str();
    let before_lines_str = subblock_ref.before_lines_str();
    let cursor_line = subblock_ref.cursor_line.trim_end().to_string();
    let cursor_line_is_empty = cursor_line.replace(" ", "").replace("\t", "").is_empty();

    let json_choices = choices
        .iter()
        .enumerate()
        .map(|(i, x)| {
            if DEBUG {
                info!("unprocessed {i} response_n_choice\n{}", x);
            }
            if finish_reasons[i] == FinishReason::Stop && !x.contains("```") {
                warn!("completion refused: no code block found in the model response");
                return json!({
                    "index": i,
                    "code_completion": "",
                    "finish_reason": finish_reasons[i].to_json_val()
                });
            }

            let mut cc = x.clone();
            if let Some(last_fenced_block) = unfence_the_last_code_block(&cc) {
                cc = last_fenced_block;

                // First, we're trying to locate cursor position and remove everything above it
                let pred_lines = cc.lines().map(|x| x.to_string()).collect::<Vec<_>>();
                let cursor_idx_mb = if !cursor_line_is_empty {
                    let cursor_matches = pred_lines
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| **x == cursor_line)
                        .map(|(idx, _)| idx)
                        .collect::<Vec<_>>();
                    if cursor_matches.len() != 1 { None } else { cursor_matches.get(0).copied() }
                } else { None };

                if let Some(idx) = cursor_idx_mb {
                    cc = pred_lines[idx..].join("\n")
                } else {
                    // If we don't find the cursor index, we try to cut lines by the file context
                    if !before_lines_str.trim().is_empty() {
                        if let Some(idx) = cc.find(before_lines_str.as_str()) {
                            cc = cc.split_at(idx + before_lines_str.len()).1.to_string();
                        } else if let Some(idx) = cc.find(before_lines_str.trim()) {
                            cc = cc.split_at(idx + before_lines_str.trim().len()).1.to_string();
                        } else {
                            let text_to_remove_lines = before_lines_str.lines().map(|x| x.to_string()).collect::<Vec<_>>();
                            let pred_lines_stripped = skip_similar_rows(&pred_lines, &text_to_remove_lines);
                            if pred_lines.len() == pred_lines_stripped.len() {
                                warn!("couldn't cut the prefix part from the predicted code, return an empty completion");
                                return json!({
                                    "index": i,
                                    "code_completion": "",
                                    "finish_reason": finish_reasons[i].to_json_val()
                                })
                            }
                            cc = pred_lines_stripped.join("\n");
                        }
                    }
                }
            } else {
                warn!("no code blocks found in the model reply, return an empty completion");
                return json!({
                    "index": i,
                    "code_completion": "",
                    "finish_reason": finish_reasons[i].to_json_val()
                })
            }

            // vscode cannot correctly handle a completion if it has spaces in front of it
            if !cursor_line_is_empty {
                let cursor_line = subblock_ref.cursor_line.replace("\n", "").replace("\r", "");
                let cc_before = cc.clone();
                cc = if let Some(idx) = cc.find(&cursor_line) {
                    cc.split_at(idx + cursor_line.len()).1.to_string()
                } else {
                    skip_similar_letters(cursor_line.as_str(), cc.as_str())
                };
                if !cursor_line.trim().is_empty() && cc == cc_before {
                    warn!("couldn't cut the cursor prefix line, return an empty completion");
                    return json!({
                        "index": i,
                        "code_completion": "",
                        "finish_reason": finish_reasons[i].to_json_val()
                    })
                }
            }

            // Removing the suffix
            if !after_lines_str.trim().is_empty() {
                if let Some(idx) = cc.find(after_lines_str.as_str()) {
                    cc = cc.split_at(idx).0.to_string();
                } else if let Some(idx) = cc.find(after_lines_str.trim()) {
                    cc = cc.split_at(idx).0.to_string();
                } else {
                    let pred_lines = cc.lines().rev().map(|x| x.to_string()).collect::<Vec<_>>();
                    let text_to_remove_lines = after_lines_str.lines().rev().map(|x| x.to_string()).collect::<Vec<_>>();
                    let pred_lines_stripped = skip_similar_rows(&pred_lines, &text_to_remove_lines).iter().rev().cloned().collect::<Vec<_>>();
                    cc = pred_lines_stripped.join("\n");
                }
            }

            let predicted_single_line = cc.matches("\n").count() == 1;
            if !is_multiline || predicted_single_line {
                if let Some(x) = cc.find("\n") {
                    cc = cc.split_at(x).0.to_string();
                }
            }
            cc = cc.replace("\r", "");

            // Instruct-based models love to add weird comments
            // Trying to remove some of them with a simple heuristics
            if !is_multiline || predicted_single_line {
                if let Some(new_row) = cc.split(" //").next() {
                    if cc.starts_with(new_row) {
                        cc = new_row.to_string();
                    }
                }
                if let Some(new_row) = cc.split("  #").next() {
                    if cc.starts_with(new_row) {
                        cc = new_row.to_string();
                    }
                }
            }

            if i == 0 {
                data4cache.completion0_text = cc.clone();
                data4cache.completion0_finish_reason = finish_reasons[i].to_string();
            }
            json!({
                "index": i,
                "code_completion": cc,
                "finish_reason": finish_reasons[i].to_json_val()
            })
        })
        .collect::<Vec<_>>();
    if DEBUG {
        info!("response_n_choices\n{:?}", json_choices);
    }
    json_choices
}

pub struct CodeCompletionReplaceScratchpad {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,

    pub token_bos: String,
    pub token_esc: String,
    pub keyword_syst: String,
    pub keyword_user: String,
    pub keyword_asst: String,

    pub new_line_symbol: Option<String>,
    pub cursor_subblock: Option<SubBlock>,
    pub context_used: Value,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

impl CodeCompletionReplaceScratchpad {
    pub fn new(
        tokenizer: Option<Arc<Tokenizer>>,
        post: &CodeCompletionPost,
        cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
        tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
        ast_service: Option<Arc<AMutex<AstIndexService>>>,
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Self {
        let data4cache = completion_cache::CompletionSaveToCache::new(cache_arc, &post);
        let data4snippet = snippets_collection::SaveSnippet::new(tele_storage, &post);
        CodeCompletionReplaceScratchpad {
            t: HasTokenizerAndEot::new(tokenizer),
            post: post.clone(),
            token_bos: "".to_string(),
            token_esc: "".to_string(),
            keyword_syst: "".to_string(),
            keyword_user: "".to_string(),
            keyword_asst: "".to_string(),
            new_line_symbol: None,
            cursor_subblock: None,
            context_used: json!({}),
            data4cache,
            data4snippet,
            ast_service,
            global_context,
        }
    }

    fn cleanup_prompt(&mut self, text: &String) -> String {
        text.replace(&self.token_bos, "")
            .replace(&self.token_esc, "")
            .replace(&self.keyword_syst, "")
            .replace(&self.keyword_user, "")
            .replace(&self.keyword_asst, "")
            .replace(&self.t.eos, "")
            .replace(&self.t.eot, "")
    }
}

#[async_trait]
impl ScratchpadAbstract for CodeCompletionReplaceScratchpad {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String> {
        self.token_bos = patch
            .get("token_bos")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        self.token_esc = patch
            .get("token_esc")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        self.keyword_syst = patch
            .get("keyword_system")
            .and_then(|x| x.as_str())
            .unwrap_or("SYSTEM:")
            .to_string();
        self.keyword_user = patch
            .get("keyword_user")
            .and_then(|x| x.as_str())
            .unwrap_or("USER:")
            .to_string();
        self.keyword_asst = patch
            .get("keyword_assistant")
            .and_then(|x| x.as_str())
            .unwrap_or("ASSISTANT:")
            .to_string();
        self.t.eot = patch
            .get("eot")
            .and_then(|x| x.as_str())
            .unwrap_or("<|endoftext|>")
            .to_string();
        self.t.eos = patch
            .get("eos")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        self.t.context_format = patch
            .get("context_format")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        self.t.rag_ratio = patch
            .get("rag_ratio")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.5);
        if self.t.tokenizer.is_some() {
            if !self.token_bos.is_empty() {
                self.t.assert_one_token(&self.token_bos.as_str())?;
            }
            if !self.token_esc.is_empty() {
                self.t.assert_one_token(&self.token_esc.as_str())?;
            }
            if !self.t.eot.is_empty() {
                self.t.assert_one_token(&self.t.eot.as_str())?;
            }
            if !self.t.eos.is_empty() {
                self.t.assert_one_token(&self.t.eos.as_str())?;
            }
        }
        Ok(())
    }

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let (n_ctx, _gcx) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.n_ctx, ccx_locked.global_context.clone())
        };
        let completion_t0 = Instant::now();
        let use_rag = self.t.rag_ratio > 0.0 && self.post.use_ast && self.ast_service.is_some();
        sampling_parameters_to_patch.max_new_tokens = MAX_NEW_TOKENS;
        sampling_parameters_to_patch.temperature = if !self.post.no_cache { Some(TEMPERATURE_INITIAL) } else { Some(TEMPERATURE_NOCACHE) };
        sampling_parameters_to_patch.stop = vec![self.t.eot.clone()];
        let cpath = crate::files_correction::canonical_path(&self.post.inputs.cursor.file);
        let source = self
            .post
            .inputs
            .sources
            .get(&self.post.inputs.cursor.file)
            .ok_or("Cursor is in file not found in sources".to_string())?
            .clone();
        let mut prompt = self.token_bos.clone();
        prompt.push_str(self.keyword_syst.as_str());
        if let Some(comment) = retrieve_a_comment(&source, &cpath, &self.post.inputs.cursor) {
            prompt.push_str(&SYSTEM_PROMPT_USERS_INTENTION.replace("<comment>", &comment));
        } else {
            prompt.push_str(SYSTEM_PROMPT);
        }
        prompt.push_str(self.token_esc.as_str());

        let mut available_tokens = n_ctx.saturating_sub(self.t.count_tokens(prompt.as_str())? as usize);
        let rag_tokens_n = if use_rag {
            let rag_tokens_n = if self.post.rag_tokens_n > 0 {
                self.post.rag_tokens_n
            } else {
                ((available_tokens as f64 * self.t.rag_ratio) as usize).max(50)
            };
            available_tokens = available_tokens.saturating_sub(rag_tokens_n);
            rag_tokens_n
        } else {
            0
        };
        available_tokens = available_tokens.saturating_sub(2 + 2 * self.t.count_tokens(self.keyword_user.as_str())? as usize);
        available_tokens = available_tokens.saturating_sub(1 + self.t.count_tokens(self.keyword_asst.as_str())? as usize);
        let subblock_required_tokens = SUBBLOCK_REQUIRED_TOKENS;
        let cursor_file_available_tokens = available_tokens.saturating_sub(subblock_required_tokens);
        if cursor_file_available_tokens <= CURSORFILE_MIN_TOKENS {
            return Err(format!("not enough tokens for the cursor file: {cursor_file_available_tokens} <= {CURSORFILE_MIN_TOKENS}"));
        }

        let text = Rope::from_str(&*self.cleanup_prompt(&source));
        let (file_content, _, (line1, line2)) = prepare_cursor_file(
            &self.t,
            cursor_file_available_tokens,
            &cpath,
            &text,
            &self.post.inputs.cursor,
        )?;
        let (subblock, _) = prepare_subblock(
            self.ast_service.clone(),
            &self.t,
            subblock_required_tokens,
            &cpath,
            &text,
            &self.post.inputs.cursor,
            MAX_ROWS_UP_OR_DOWNS,
            MIN_ROWS_TO_SKIP_CARET
        ).await?;
        if use_rag {
            let pp_settings = {
                let ccx_locked = ccx.lock().await;
                ccx_locked.postprocess_parameters.clone()
            };
            let extra_context = retrieve_ast_based_extra_context(
                self.global_context.clone(),
                self.ast_service.clone(),
                &self.t,
                &cpath,
                &self.post.inputs.cursor,
                (line1 as i32, line2 as i32),
                pp_settings,
                rag_tokens_n,
                &mut self.context_used
            ).await;
            prompt.push_str(self.keyword_user.as_str());
            prompt.push_str(extra_context.as_str());
            prompt.push_str(self.token_esc.as_str());
        }
        self.cursor_subblock = Some(subblock);
        self.new_line_symbol = if self.cursor_subblock.as_ref().unwrap().cursor_line.ends_with("\r\n") {
            Some("\r\n".to_string())
        } else {
            Some("\n".to_string())
        };
        // Editing file and the subblock within it to rewrite by the model
        prompt.push_str(self.keyword_user.as_str());
        prompt.push_str(format!("{file_content}\n{}", self.cursor_subblock.as_ref().unwrap().prompt()?).as_str());
        prompt.push_str(self.token_esc.as_str());

        let completion_ms = completion_t0.elapsed().as_millis() as i32;
        self.context_used["fim_ms"] = Value::from(completion_ms);
        self.context_used["n_ctx".to_string()] = Value::from(n_ctx as i64);
        self.context_used["rag_tokens_limit".to_string()] = Value::from(rag_tokens_n as i64);
        info!(" -- /post completion {}ms-- ", completion_ms);

        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!(
                "chat re-encode whole prompt again gives {} tokens",
                self.t.count_tokens(prompt.as_str())?
            );
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        let json_choices = process_n_choices(
            &mut self.cursor_subblock,
            &choices,
            &finish_reasons,
            self.post.inputs.multiline,
            &mut self.data4cache,
        );
        snippets_collection::snippet_register_from_data4cache(
            &self.data4snippet,
            &mut self.data4cache,
            self.context_used != json!({}),
        );
        Ok(json!(
            {
                "choices": json_choices,
                "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
                "model": self.post.model.clone(),
                "context": self.context_used,
            }
        ))
    }

    fn response_streaming(
        &mut self,
        _delta: String,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_message_streaming(
        &mut self,
        _delta: &Value,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String> {
        Ok(vec![])
    }

    fn streaming_finished(&mut self, _finish_reason: FinishReason) -> Result<Value, String> {
        Err("not implemented".to_string())
    }
}

pub struct CodeCompletionReplacePassthroughScratchpad {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub new_line_symbol: Option<String>,
    pub cursor_subblock: Option<SubBlock>,
    pub context_used: Value,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

impl CodeCompletionReplacePassthroughScratchpad {
    pub fn new(
        tokenizer: Option<Arc<Tokenizer>>,
        post: &CodeCompletionPost,
        cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
        tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
        ast_service: Option<Arc<AMutex<AstIndexService>>>,
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Self {
        let data4cache = completion_cache::CompletionSaveToCache::new(cache_arc, &post);
        let data4snippet = snippets_collection::SaveSnippet::new(tele_storage, &post);
        CodeCompletionReplacePassthroughScratchpad {
            t: HasTokenizerAndEot::new(tokenizer),
            post: post.clone(),
            new_line_symbol: None,
            cursor_subblock: None,
            context_used: json!({}),
            data4cache,
            data4snippet,
            ast_service,
            global_context,
        }
    }
}

#[async_trait]
impl ScratchpadAbstract for CodeCompletionReplacePassthroughScratchpad {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String> {
        self.t.context_format = patch
            .get("context_format")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        self.t.rag_ratio = patch
            .get("rag_ratio")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.5);
        Ok(())
    }

    async fn prompt(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let (n_ctx, gcx) = {
            let ccx_locked = ccx.lock().await;
            (ccx_locked.n_ctx, ccx_locked.global_context.clone())
        };
        let caps = gcx.read().await.caps.clone().ok_or_else(|| "No caps".to_string())?;
        let completion_t0 = Instant::now();
        let use_rag = self.t.rag_ratio > 0.0 && self.post.use_ast && self.ast_service.is_some();
        sampling_parameters_to_patch.max_new_tokens = MAX_NEW_TOKENS;
        sampling_parameters_to_patch.temperature = if !self.post.no_cache { Some(TEMPERATURE_INITIAL) } else { Some(TEMPERATURE_NOCACHE) };
        sampling_parameters_to_patch.stop = vec![]; // avoid model cutting completion too early 
        let cpath = crate::files_correction::canonical_path(&self.post.inputs.cursor.file);
        let source = self
            .post
            .inputs
            .sources
            .get(&self.post.inputs.cursor.file)
            .ok_or("Cursor is in file not found in sources".to_string())?
            .clone();

        let mut messages = vec![];
        if let Some(comment) = retrieve_a_comment(&source, &cpath, &self.post.inputs.cursor) {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(
                    SYSTEM_PROMPT_USERS_INTENTION.replace("<comment>", &comment),
                ),
                tool_calls: None,
                tool_call_id: "".to_string(),
                ..Default::default()
            });
        } else {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(SYSTEM_PROMPT.to_string()),
                ..Default::default()
            });
        }
        let mut available_tokens = n_ctx.saturating_sub(
            self.t.count_tokens(&messages[0].content.content_text_only())? as usize + 3,
        );
        let rag_tokens_n = if use_rag {
            let rag_tokens_n = if self.post.rag_tokens_n > 0 {
                self.post.rag_tokens_n
            } else {
                ((available_tokens as f64 * self.t.rag_ratio) as usize).max(50)
            };
            available_tokens = available_tokens.saturating_sub(rag_tokens_n);
            rag_tokens_n
        } else {
            0
        };
        let subblock_required_tokens = SUBBLOCK_REQUIRED_TOKENS;
        let cursor_file_available_tokens = available_tokens.saturating_sub(subblock_required_tokens);
        if cursor_file_available_tokens <= CURSORFILE_MIN_TOKENS {
            return Err(format!("not enough tokens for the cursor file: {cursor_file_available_tokens} <= {CURSORFILE_MIN_TOKENS}"));
        }

        let text = Rope::from_str(&*source);
        let (file_content, _file_content_tokens_count, (line1, line2)) = prepare_cursor_file(
            &self.t,
            cursor_file_available_tokens,
            &cpath,
            &text,
            &self.post.inputs.cursor,
        )?;
        let (subblock, _subblock_tokens_count) = prepare_subblock(
            self.ast_service.clone(),
            &self.t,
            subblock_required_tokens,
            &cpath,
            &text,
            &self.post.inputs.cursor,
            MAX_ROWS_UP_OR_DOWNS,
            MIN_ROWS_TO_SKIP_CARET
        ).await?;
        if use_rag {
            let pp_settings = {
                let ccx_locked = ccx.lock().await;
                ccx_locked.postprocess_parameters.clone()
            };
            let extra_context = retrieve_ast_based_extra_context(
                self.global_context.clone(),
                self.ast_service.clone(),
                &self.t,
                &cpath,
                &self.post.inputs.cursor,
                (line1 as i32, line2 as i32),
                pp_settings,
                rag_tokens_n,
                &mut self.context_used
            ).await;
            if !extra_context.is_empty() {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::SimpleText(extra_context),
                    ..Default::default()
                });
            }
        }
        self.cursor_subblock = Some(subblock);
        self.new_line_symbol = if self.cursor_subblock.as_ref().unwrap().cursor_line.ends_with("\r\n") {
            Some("\r\n".to_string())
        } else {
            Some("\n".to_string())
        };
        // Editing file and the subblock within it to rewrite by the model
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(format!(
                "{file_content}\n{}",
                self.cursor_subblock.as_ref().unwrap().prompt()?
            )),
            ..Default::default()
        });

        let model = resolve_completion_model(caps.clone(), &self.post.model, true)?;
        let json_messages = &serde_json::to_string(&json!({
            "messages":  messages.iter().map(|x| { x.into_value(&None, &model.base.id) }).collect::<Vec<_>>(),
        }))
        .unwrap();
        let prompt = format!("PASSTHROUGH {json_messages}").to_string();

        let completion_ms = completion_t0.elapsed().as_millis() as i32;
        self.context_used["fim_ms"] = Value::from(completion_ms);
        self.context_used["n_ctx".to_string()] = Value::from(n_ctx as i64);
        self.context_used["rag_tokens_limit".to_string()] = Value::from(rag_tokens_n as i64);
        info!(" -- /post completion {}ms-- ", completion_ms);

        if DEBUG {
            info!(
                "chat re-encode whole prompt again gives {} tokens",
                self.t.count_tokens(prompt.as_str())?
            );
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        _choices: Vec<String>,
        _finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        Err("not implemented".to_string())
    }

    fn response_streaming(
        &mut self,
        _delta: String,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_message_n_choices(
        &mut self,
        choices: Vec<String>,
        finish_reasons: Vec<FinishReason>,
    ) -> Result<Value, String> {
        let json_choices = process_n_choices(
            &mut self.cursor_subblock,
            &choices,
            &finish_reasons,
            self.post.inputs.multiline,
            &mut self.data4cache,
        );
        snippets_collection::snippet_register_from_data4cache(
            &self.data4snippet,
            &mut self.data4cache,
            self.context_used != json!({}),
        );
        Ok(json!({
            "choices": json_choices,
            "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
            "model": self.post.model.clone(),
            "context": self.context_used,
        }))
    }

    fn response_message_streaming(
        &mut self,
        _json: &Value,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String> {
        Ok(vec![])
    }

    fn streaming_finished(&mut self, _finish_reason: FinishReason) -> Result<Value, String> {
        Err("not implemented".to_string())
    }
}
