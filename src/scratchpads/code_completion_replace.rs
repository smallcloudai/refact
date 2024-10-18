use crate::ast::ast_indexer_thread::AstIndexService;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{CodeCompletionPost, CursorPosition, SamplingParameters};
use crate::completion_cache;
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::telemetry::snippets_collection;
use crate::telemetry::telemetry_structs;
use async_trait::async_trait;
use ropey::Rope;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::vec;
use tokenizers::Tokenizer;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

const DEBUG: bool = false;
const SYSTEM_PROMPT: &str = r#"You are given an incomplete code file and a block of code from that file. Within this block, an unfinished line is marked with <CURSOR>. Your task is to complete the code at the <CURSOR> position.
Ensure you copy the additional lines from before and after the <CURSOR> line exactly as they are. 
Do not comment the new code you added!"#;
const SUBBLOCK_CUT_TOKENS_N: usize = 3;

#[derive(Debug, Clone)] 
pub struct SubBlock {
    before_lines: Vec<String>,
    cursor_line: String,
    after_lines: Vec<String>,
    cut_part: Option<String>
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
    pub _ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub _global_context: Arc<ARwLock<GlobalContext>>,
}

impl SubBlock {
    fn prompt(&self, tokenizer: &HasTokenizerAndEot) -> Result<String, String> {
        let mut code = self.before_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("");

        let (new_cursor_line, _cut_part) = if !self.cursor_line.is_empty() {
            let tokenizer_ref = tokenizer.tokenizer
                .write()
                .map_err(|x| x.to_string())?;
            let cursor_line = self.cursor_line
                .replace("\r\n", "\n")
                .strip_suffix("\n")
                .unwrap_or(&self.cursor_line)
                .to_string();
            let cursor_line_tokens = tokenizer_ref.encode(&*cursor_line, false)
                .map_err(|x| x.to_string())?;
            let cut_until = cursor_line_tokens.len().saturating_sub(SUBBLOCK_CUT_TOKENS_N);
            (tokenizer_ref.decode(&cursor_line_tokens.get_ids()[..cut_until], true)
                 .map_err(|x| x.to_string())?,
             tokenizer_ref.decode(&cursor_line_tokens.get_ids()[cut_until..], true)
                 .map_err(|x| x.to_string())?)
        } else {
            (self.cursor_line.clone(), "".to_string())
        };
        code.push_str(format!("{}<CURSOR>\n", new_cursor_line).as_str());
        code.push_str(self.after_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("")
            .as_str());
        let extra_user_message = format!("The user started to type this, use it as a prompt:\n```\n{}\n```", self.cursor_line);
        Ok(format!("# Block of code:\n```\n{code}\n```\n{extra_user_message}"))
    }

    fn prefilling_prompt(&mut self, tokenizer: &HasTokenizerAndEot) -> Result<String, String> {
        let mut code = self.before_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("");
        let (new_cursor_line, cut_part) = if !self.cursor_line.is_empty() {
            let tokenizer_ref = tokenizer.tokenizer
                .write()
                .map_err(|x| x.to_string())?;
            let cursor_line = self.cursor_line
                .replace("\r\n", "\n")
                .strip_suffix("\n")
                .unwrap_or(&self.cursor_line)
                .to_string();
            let cursor_line_tokens = tokenizer_ref.encode(&*cursor_line, false)
                .map_err(|x| x.to_string())?;
            let cut_until = cursor_line_tokens.len().saturating_sub(SUBBLOCK_CUT_TOKENS_N);
            (tokenizer_ref.decode(&cursor_line_tokens.get_ids()[..cut_until], true)
                 .map_err(|x| x.to_string())?,
             tokenizer_ref.decode(&cursor_line_tokens.get_ids()[cut_until..], true)
                 .map_err(|x| x.to_string())?)
        } else {
            (self.cursor_line.clone(), "".to_string())
        };
        code.push_str(&new_cursor_line);
        self.cut_part = Some(cut_part);
        Ok(format!("# Completed block of code:\n```\n{code}"))
    }
    
    fn after_lines_str(&self) -> String {
        self.after_lines
            .iter()
            .map(|x| x.replace("\r\n", "\n"))
            .collect::<Vec<_>>()
            .join("")
    }
}

fn prepare_main_file(
    tokenizer: &HasTokenizerAndEot,
    max_tokens: usize,
    file_name: &PathBuf,
    file_text: &Rope,
    cursor_pos: &CursorPosition,
) -> Result<(String, usize), String> {
    let mut output_lines: VecDeque<String> = VecDeque::new();
    let mut tokens_used: usize = 0;
    let mut line_idx_offset: i32 = 1;

    if let Some(line) = file_text.line(cursor_pos.line as usize).as_str() {
        output_lines.push_front(line.to_string());
        tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
        if tokens_used > max_tokens {
            return Err("Tokens limit is too small to fit the main file".to_string());
        }
    } else {
        return Err("Cannot retrieve the cursor line from the given file".to_string());
    }
    loop {
        if cursor_pos.line - line_idx_offset >= 0 {
            let line = file_text.line((cursor_pos.line - line_idx_offset) as usize);
            if let Some(line) = line.as_str() {
                tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
                if tokens_used > max_tokens {
                    break;
                }
                output_lines.push_front(line.to_string());
            }
        }
        if cursor_pos.line + line_idx_offset < file_text.len_lines() as i32 {
            let line = file_text.line((cursor_pos.line + line_idx_offset) as usize);
            if let Some(line) = line.as_str() {
                tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
                if tokens_used > max_tokens {
                    break;
                }
                output_lines.push_back(line.to_string());
            }
        }

        if cursor_pos.line - line_idx_offset < 0 && cursor_pos.line + line_idx_offset >= file_text.len_lines() as i32  {
            break;
        }
        
        line_idx_offset += 1;
    }
    let file_text = output_lines
        .into_iter()
        .map(|x| x.replace("\r\n", "\n"))
        .collect::<Vec<_>>().join("");
    let data = format!("File name:\n{}\nContent:\n```\n{file_text}\n```", file_name.to_string_lossy());
    let tokens_used = tokenizer.count_tokens(&data).unwrap_or(0) as usize;
    Ok((data, tokens_used))
}

fn prepare_subblock(
    tokenizer: &HasTokenizerAndEot,
    max_tokens: usize,
    file_text: &Rope,
    cursor_pos: &CursorPosition,
) -> Result<(SubBlock, usize), String> {
    let mut subblock: SubBlock = SubBlock {
        before_lines: vec![],
        cursor_line: String::new(),
        after_lines: vec![],
        cut_part: None
    };
    let mut tokens_used: usize = 0;

    if let Some(line) = file_text.line(cursor_pos.line as usize).as_str() {
        subblock.cursor_line = line.to_string();
        tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
        if tokens_used > max_tokens {
            return Err("Tokens limit is too small to fit the code subblock".to_string());
        }
    } else {
        return Err("Cannot retrieve the cursor line from the given file".to_string());
    }

    for i in cursor_pos.line - 4..cursor_pos.line {
        if i >= 0 {
            if let Some(line) = file_text.line(i as usize).as_str() {
                subblock.before_lines.push(line.to_string());
                tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
                if tokens_used > max_tokens {
                    return Err("Tokens limit is too small to fit the context for the code subblock".to_string());
                }
            }
        }
    }

    for i in cursor_pos.line + 1..cursor_pos.line + 5 {
        if i < file_text.len_lines() as i32 {
            let line = file_text.line(i as usize);
            if let Some(line) = line.as_str() {
                tokens_used += tokenizer.count_tokens(line).unwrap_or(0) as usize;
                if tokens_used > max_tokens {
                    break;
                }
                subblock.after_lines.push(line.to_string());
            }
        }
    }
    Ok((subblock, tokens_used))
}


impl CodeCompletionReplaceScratchpad {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
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
            _ast_service: ast_service,
            _global_context: global_context,
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
        _exploration_tools: bool,
        _agentic_tools: bool,
    ) -> Result<(), String> {
        self.token_bos = patch.get("token_bos").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.token_esc = patch.get("token_esc").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.keyword_syst = patch.get("keyword_system").and_then(|x| x.as_str()).unwrap_or("SYSTEM:").to_string();
        self.keyword_user = patch.get("keyword_user").and_then(|x| x.as_str()).unwrap_or("USER:").to_string();
        self.keyword_asst = patch.get("keyword_assistant").and_then(|x| x.as_str()).unwrap_or("ASSISTANT:").to_string();
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.eos = patch.get("eos").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.t.context_format = patch.get("context_format").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        self.t.rag_ratio = patch.get("rag_ratio").and_then(|x| x.as_f64()).unwrap_or(0.5);
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
        // let use_rag = !self.t.context_format.is_empty() && self.t.rag_ratio > 0.0 && self.post.use_ast && self.ast_service.is_some();
        sampling_parameters_to_patch.max_new_tokens = 256;
        sampling_parameters_to_patch.temperature = Some(0.05);
        sampling_parameters_to_patch.stop = vec![self.t.eot.clone()];
        if !self.post.inputs.multiline {
            sampling_parameters_to_patch.stop.push("\n".to_string());
        }
        let mut prompt = self.token_bos.clone();
        prompt.push_str(self.keyword_syst.as_str());
        prompt.push_str(SYSTEM_PROMPT);
        prompt.push_str(self.token_esc.as_str());

        let mut available_tokens = n_ctx.saturating_sub(self.t.count_tokens(prompt.as_str())? as usize);
        // let mut rag_tokens_n = if self.post.rag_tokens_n > 0 {
        //     self.post.rag_tokens_n.min(4096).max(50)
        // } else {
        //     ((available_tokens as f64 * self.t.rag_ratio) as usize).min(4096).max(50)
        // };
        // available_tokens = available_tokens.saturating_sub(rag_tokens_n);
        available_tokens = available_tokens.saturating_sub(1 + self.t.count_tokens(self.keyword_user.as_str())? as usize);
        available_tokens = available_tokens.saturating_sub(1 + self.t.count_tokens(self.keyword_asst.as_str())? as usize);
        let main_file_available_tokens = (available_tokens as f64 * 0.9) as usize;
        let subblock_available_tokens = available_tokens.saturating_sub(main_file_available_tokens).min(256).max(32);

        let cpath = crate::files_correction::canonical_path(&self.post.inputs.cursor.file);
        let mut source = self.post.inputs.sources.get(
            &self.post.inputs.cursor.file
        ).ok_or("Cursor is in file not found in sources".to_string())?.clone();
        source = self.cleanup_prompt(&source);
        let text = Rope::from_str(&*source);

        let (file_content, _file_content_tokens_count) = prepare_main_file(
            &self.t,
            main_file_available_tokens,
            &cpath,
            &text,
            &self.post.inputs.cursor,
        )?;
        let (subblock, _subblock_tokens_count) = prepare_subblock(
            &self.t,
            subblock_available_tokens,
            &text,
            &self.post.inputs.cursor,
        )?;
        self.cursor_subblock = Some(subblock);
        self.new_line_symbol = if self.cursor_subblock.as_ref().unwrap().cursor_line.ends_with("\r\n") {
            Some("\r\n".to_string())
        } else {
            Some("\n".to_string())
        };
        prompt.push_str(self.keyword_user.as_str());
        prompt.push_str(format!("{file_content}\n{}", self.cursor_subblock.as_ref().unwrap().prompt(&self.t)?).as_str());
        prompt.push_str(self.token_esc.as_str());
        prompt.push_str(self.keyword_asst.as_str());
        prompt.push_str(self.cursor_subblock.as_mut().unwrap().prefilling_prompt(&self.t)?.as_str());

        if DEBUG {
            info!("chat prompt\n{}", prompt);
            info!("chat re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        stopped: Vec<bool>,
    ) -> Result<Value, String> {
        let subblock_ref = self.cursor_subblock
            .as_mut()
            .expect("cursor_subblock must be initialized in the prompt");
        let cut_part = subblock_ref.cut_part.clone().expect("cut_part must be initialized in the prompt");
        let after_lines_str = subblock_ref.after_lines_str();
        let json_choices = choices.iter().enumerate().map(|(i, x)| {
            if DEBUG {
                info!("unprocessed {i} response_n_choice\n{:?}", x);
            }

            let (mut cc, mut finished) = _cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline);
            cc = cc.replacen(cut_part.as_str(), "", 1);
            if !after_lines_str.trim().is_empty() {
                if let Some(idx) = cc.find(after_lines_str.as_str()) {
                    cc = cc.split_at(idx).0.to_string();
                } else if let Some(idx) = cc.find(after_lines_str.trim()) {
                    cc = cc.split_at(idx).0.to_string();
                }
            }
            finished |= stopped[i];
            let finish_reason = if finished {
                cc = cc.trim_end().to_string();
                "stop"
            } else {
                "length"
            }.to_string();
            if i == 0 {
                self.data4cache.completion0_text = cc.clone();
                self.data4cache.completion0_finish_reason = finish_reason.clone();
            }
            json!({
                "index": i,
                "code_completion": cc,
                "finish_reason": finish_reason.clone(),
            })
        }).collect::<Vec<_>>();
        if DEBUG {
            info!("response_n_choices\n{:?}", json_choices);
        }

        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache, self.context_used != json!({}));
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
        _stop_toks: bool,
        _stop_length: bool,
    ) -> Result<(Value, bool), String> {
        Err("".to_string())
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String> {
        Err("".to_string())
    }
}

fn _cut_result(text: &str, eot_token: &str, multiline: bool) -> (String, bool) {
    let mut cut_at = vec![];
    if let Some(x) = text.find(eot_token) {
        cut_at.push(x);
    }
    if let Some(x) = text.find("\r\n\r\n") {
        cut_at.push(x);
    }
    if let Some(x) = text.find("```") {
        cut_at.push(x);
    }
    if !multiline {
        if let Some(x) = text.find("\n") {
            cut_at.push(x);
        }
    }
    if cut_at.is_empty() {
        return (text.to_string().replace("\r", ""), false);
    }
    let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    let ans = text.split_at(cut_at).0.to_string();
    (ans.replace("\r", ""), true)
}
