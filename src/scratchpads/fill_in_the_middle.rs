use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Instant;
use std::vec;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use ropey::Rope;
use serde_json::{Value, json};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tracing::info;
use std::collections::HashSet;

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::ast::ast_structs::{AstDB, AstDefinition};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{CodeCompletionPost, ContextFile, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::completion_cache;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::telemetry::snippets_collection;
use crate::telemetry::telemetry_structs;


const DEBUG: bool = false;
const TAKE_USAGES_AROUND_CURSOR: usize = 20;


pub struct FillInTheMiddleScratchpad {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub order: String,
    pub fim_prefix: String,
    pub fim_suffix: String,
    pub fim_middle: String,
    pub context_used: Value,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

impl FillInTheMiddleScratchpad {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: &CodeCompletionPost,
        order: String,
        cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
        tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
        ast_service: Option<Arc<AMutex<AstIndexService>>>,
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Self {
        let data4cache = completion_cache::CompletionSaveToCache::new(cache_arc, &post);
        let data4snippet = snippets_collection::SaveSnippet::new(tele_storage, &post);
        FillInTheMiddleScratchpad {
            t: HasTokenizerAndEot::new(tokenizer),
            post: post.clone(),
            order,
            fim_prefix: String::new(),
            fim_suffix: String::new(), fim_middle: String::new(),
            context_used: json!({}),
            data4cache,
            data4snippet,
            ast_service,
            global_context,
        }
    }

    fn cleanup_prompt(&mut self, text: &String) -> String {
        text.replace(&self.fim_prefix, "")
            .replace(&self.fim_middle, "")
            .replace(&self.fim_suffix, "")
            .replace(&self.t.eos, "")
            .replace(&self.t.eot, "")
    }
}

fn add_context_to_prompt(
    context_format: &String,
    prompt: &String,
    postprocessed_messages: &Vec<ContextFile>,
) -> String {
    let mut context_files = vec![];
    if context_format == "starcoder" {
        for m in postprocessed_messages {
            let s = format!(
                "{}{}{}{}",
                "<file_sep>",
                m.file_name,
                "\n",
                m.file_content
            );
            context_files.push(s);
        }
        if !context_files.is_empty() {
            context_files.insert(0, "<repo_name>default_repo".to_string());
            context_files.push("<file_sep>".to_string())
        }
        format!(
            "{}{}",
            context_files.join(""),
            prompt,
        )
    } else {
        tracing::warn!("context_format \"{}\" not recognized", context_format);
        return prompt.clone();
    }
}

#[async_trait]
impl ScratchpadAbstract for FillInTheMiddleScratchpad {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
        _exploration_tools: bool,
        _agentic_tools: bool,
    ) -> Result<(), String> {
        // That will work for some models (starcoder) without patching
        self.fim_prefix = patch.get("fim_prefix").and_then(|x| x.as_str()).unwrap_or("<fim_prefix>").to_string();
        self.fim_suffix = patch.get("fim_suffix").and_then(|x| x.as_str()).unwrap_or("<fim_suffix>").to_string();
        self.fim_middle = patch.get("fim_middle").and_then(|x| x.as_str()).unwrap_or("<fim_middle>").to_string();
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.eos = patch.get("eos").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.t.context_format = patch.get("context_format").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        self.t.rag_ratio = patch.get("rag_ratio").and_then(|x| x.as_f64()).unwrap_or(0.5);
        self.t.assert_one_token(&self.fim_prefix.as_str())?;
        self.t.assert_one_token(&self.fim_suffix.as_str())?;
        self.t.assert_one_token(&self.fim_middle.as_str())?;
        self.t.assert_one_token(&self.t.eot.as_str())?;
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
        let n_ctx = ccx.lock().await.n_ctx;
        let fim_t0 = Instant::now();
        let use_rag = !self.t.context_format.is_empty() && self.t.rag_ratio > 0.0 && self.post.use_ast && self.ast_service.is_some();
        let mut rag_tokens_n = if self.post.rag_tokens_n > 0 {
            self.post.rag_tokens_n.min(4096).max(50)
        } else {
            ((n_ctx as f64 * self.t.rag_ratio) as usize).min(4096).max(50)
        };
        if !use_rag {
            rag_tokens_n = 0;
        }
        if !use_rag && self.post.use_ast {
            tracing::warn!("will not use ast because {}{}{}{}", self.t.context_format.is_empty() as i32, self.post.use_ast as i32, (rag_tokens_n > 0) as i32, self.ast_service.is_some() as i32);
        }

        let limit: i32 = (n_ctx as i32) - (self.post.parameters.max_new_tokens as i32) - (rag_tokens_n as i32);
        if limit < 512 {
            let msg = format!("n_ctx={} - max_new_tokens={} - rag_tokens_n={} leaves too little {} space for completion to work",
            n_ctx, self.post.parameters.max_new_tokens, rag_tokens_n, limit);
            tracing::warn!("{}", msg);
            return Err(msg);
        }

        let cpath = crate::files_correction::canonical_path(&self.post.inputs.cursor.file);

        let supports_stop = true; // some hf models do not support stop, but it's a thing of the past?
        if supports_stop {
            let mut stop_list = vec![self.t.eot.clone(), "\n\n".to_string()];
            if !self.post.inputs.multiline {
                stop_list.push("\n".to_string());  // This doesn't stop hf inference, only whole tokens do
            }
            sampling_parameters_to_patch.stop = stop_list;
        }
        let mut source = self.post.inputs.sources.get(
                &self.post.inputs.cursor.file
            ).ok_or("Cursor is in file not found in sources".to_string())?.clone();
        source = self.cleanup_prompt(&source);

        let text = Rope::from_str(&*source);

        let pos = &self.post.inputs.cursor;
        let mut before_iter = text.lines_at(pos.line as usize).reversed();
        let mut after_iter = text.lines_at(pos.line as usize + 1);
        let mut tokens_used = 0;

        let mut before_line = before_iter.next();

        let cursor_line1: String;
        let col = pos.character as usize;
        // TODO: use get_slice and handle error
        cursor_line1 = text.line(pos.line as usize).slice(0..col).to_string();
        // UNFINISHED LI|

        let mut after_line = after_iter.next();

        let cursor_line2: String;
        if self.post.inputs.multiline {
            // TODO: use get_slice and handle error
            cursor_line2 = text.line(pos.line as usize).slice(col..).to_string();
        } else {
            cursor_line2 = "".to_string();
        }

        let mut before = vec![];
        let mut after = String::new();
        let mut fim_line1: i32 = i32::MAX;
        let mut fim_line2: i32 = i32::MIN;
        tokens_used += self.t.count_tokens(
            (cursor_line1.clone() + &cursor_line2).as_str()
        )?;
        let mut rel_line_n: i32 = 0;
        while before_line.is_some() || after_line.is_some() {
            rel_line_n += 1;
            if let Some(before_line) = before_line {
                let before_line = before_line.to_string();
                let tokens = self.t.count_tokens(before_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                before.push(before_line);
                fim_line1 = pos.line - rel_line_n as i32;
            }
            if let Some(after_line) = after_line {
                let after_line = after_line.to_string();
                let tokens = self.t.count_tokens(after_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                after.push_str(&after_line);
                fim_line2 = pos.line + rel_line_n as i32;
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }

        let before = before.into_iter().rev().collect::<Vec<_>>().join("");
        info!("{} FIM prompt {} tokens used < limit {}", crate::nicer_logs::last_n_chars(&cpath.display().to_string(), 30), tokens_used, limit);
        let mut prompt: String;
        if self.order == "PSM" {
            prompt = format!(
                "{}{}{}{}{}{}{}{}",
                self.t.eos,
                self.fim_prefix,
                before,
                cursor_line1,
                self.fim_suffix,
                cursor_line2,
                after,
                self.fim_middle
            );
        } else if self.order == "SPM" {
            prompt = format!(
                "{}{}{}{}{}{}{}{}",
                self.t.eos,
                self.fim_suffix,
                cursor_line2,
                after,
                self.fim_prefix,
                before,
                cursor_line1,
                self.fim_middle,
            );
        } else {
            return Err(format!("order \"{}\" not recognized", self.order));
        }
        let fim_ms = fim_t0.elapsed().as_millis() as i32;

        if use_rag && rag_tokens_n > 0 {
            info!(" -- rag search starts --");
            let rag_t0 = Instant::now();
            let mut ast_messages: Vec<ContextFile> = if let Some(ast) = &self.ast_service {
                let ast_index = ast.lock().await.ast_index.clone();
                _cursor_position_to_context_file(ast_index.clone(), cpath.to_string_lossy().to_string(), pos.line).await
            } else {
                vec![]
            };

            let to_buckets_ms = rag_t0.elapsed().as_millis() as i32;

            if fim_line1 != i32::MAX && fim_line2 != i32::MIN {
                let fim_ban = ContextFile {
                    file_name: cpath.to_string_lossy().to_string(),
                    file_content: "".to_string(),
                    line1: (fim_line1 + 1) as usize,
                    line2: (fim_line2 + 1) as usize,
                    symbols: vec![],
                    gradient_type: -1,
                    usefulness: -1.0,
                };
                ast_messages.push(fim_ban);
            }

            info!(" -- post processing starts --");
            let post_t0 = Instant::now();

            let mut pp_settings = {
                let ccx_locked = ccx.lock().await;
                ccx_locked.postprocess_parameters.clone()
            };
            if pp_settings.max_files_n == 0 {
                pp_settings.max_files_n = 10;
            }

            let postprocessed_messages = postprocess_context_files(
                self.global_context.clone(),
                &ast_messages,
                self.t.tokenizer.clone(),
                rag_tokens_n,
                false,
                &pp_settings,
            ).await;

            prompt = add_context_to_prompt(&self.t.context_format, &prompt, &postprocessed_messages);
            // let rag_ms = rag_t0.elapsed().as_millis() as i32;
            let post_ms = post_t0.elapsed().as_millis() as i32;
            info!("fim {}ms, buckets {}ms, post {}ms",
                fim_ms,
                to_buckets_ms, post_ms
            );

            // if was_looking_for.is_some() {
                // self.context_used = context_to_fim_debug_page(
                //     &postprocessed_messages,
                //     // &was_looking_for.unwrap()
                // );
            //     self.context_used["fim_ms"] = Value::from(fim_ms);
            //     self.context_used["rag_ms"] = Value::from(rag_ms);
            //     self.context_used["n_ctx".to_string()] = Value::from(n_ctx as i64);
            //     self.context_used["rag_tokens_limit".to_string()] = Value::from(rag_tokens_n as i64);
            // }
        }

        if DEBUG {
            info!("cursor position\n{:?}", self.post.inputs.cursor);
            info!("prompt\n{}", prompt);
            info!("re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
        }
        info!("re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        stopped: Vec<bool>
    ) -> Result<Value, String> {
        let json_choices = choices.iter().enumerate().map(|(i, x)| {
            let (mut cc, mut finished) = _cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline);
            finished |= stopped[i];
            let finish_reason = if finished {
                cc = cc.trim_end().to_string();
                "stop"
            } else {
                "length"
            }.to_string();
            if i==0 {
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
        return Ok(json!(
            {
                "choices": json_choices,
                "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
                "model": self.post.model.clone(),
                "context": self.context_used,
            }
        ));
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(Value, bool), String> {
        let mut finished;
        let json_choices;
        // info!("XXXXX delta: {:?}", delta);
        // info!("XXXXX stop_toks: {:?}", stop_toks);
        // info!("XXXXX stop_length: {:?}", stop_length);
        if !delta.is_empty() || stop_toks {
            let mut s: String;
            (s, finished) = _cut_result(&delta, self.t.eot.as_str(), self.post.inputs.multiline);
            finished |= stop_toks;
            if finished {
                // can stay consistent with trim() only if that's the final iteration
                s = s.trim_end().to_string();
                self.data4cache.completion0_finish_reason = if finished { "stop".to_string() } else { "".to_string() };
            }
            self.data4cache.completion0_text.push_str(&s);
            json_choices = json!([{
                "index": 0,
                "code_completion": s,
                "finish_reason": if finished { serde_json::Value::String("stop".to_string()) } else { serde_json::Value::Null },
            }]);
        } else {
            assert!(stop_length);
            json_choices = json!([{
                "index": 0,
                "code_completion": "",
                "finish_reason": "length"
            }]);
            self.data4cache.completion0_finish_reason = "length".to_string();
            finished = true;
        }
        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache, self.context_used != json!({}));
        let ans = json!({
            "choices": json_choices,
            "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
        });
        Ok((ans, finished))
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        return Err("".to_string());
    }
}

fn _cut_result(text: &str, eot_token: &str, multiline: bool) -> (String, bool) {
    let mut cut_at = vec![];
    if let Some(x) = text.find(eot_token) {
        cut_at.push(x);
    }
    if let Some(x) = text.find("\n\n") {
        cut_at.push(x);
    }
    if let Some(x) = text.find("\r\n\r\n") {
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
    return (ans.replace("\r", ""), true);
}

async fn _cursor_position_to_context_file(
    ast_index: Arc<AMutex<AstDB>>,
    cpath: String,
    cursor_line: i32,
) -> Vec<ContextFile> {
    if cursor_line < 0 || cursor_line > 65535 {
        tracing::error!("cursor line {} out of range", cursor_line);
        return vec![]
    }
    let cursor_line = (cursor_line + 1) as usize;  // count from 1
    let usages: Vec<(usize, String)> = crate::ast::ast_db::doc_usages(ast_index.clone(), &cpath).await;
    // uline in usage counts from 1

    let mut distances: Vec<(i32, String, usize)> = usages.into_iter().map(|(line, usage)| {
        let distance = (line as i32 - cursor_line as i32).abs();
        (distance, usage, line)
    }).collect();
    distances.sort_by_key(|&(distance, _, _)| distance);
    let nearest_usages: Vec<(usize, String)> = distances.into_iter().take(TAKE_USAGES_AROUND_CURSOR).map(|(_, usage, line)| (line, usage)).collect();

    // info!("nearest_usages={:?}", nearest_usages);
    let unique_paths: HashSet<_> = nearest_usages.into_iter().map(|(_line, double_colon_path)| double_colon_path).collect();
    let mut output = vec![];
    for double_colon_path in unique_paths {
        let defs: Vec<Arc<AstDefinition>> = crate::ast::ast_db::definitions(ast_index.clone(), double_colon_path.as_str()).await;
        if defs.len() != 1 {
            tracing::warn!("hmm, number of definitions for {} is not one {}", double_colon_path, defs.len());
        }
        for def in defs {
            output.push(ContextFile {
                file_name: def.cpath.clone(),
                file_content: "".to_string(),
                line1: def.full_range.start_point.row + 1,
                line2: def.full_range.end_point.row + 1,
                symbols: vec![double_colon_path.clone()],
                gradient_type: -1,
                usefulness: 0.,
            });
        }
    }
    info!("FIM context\n{:#?}", output);
    output
}


// pub fn context_to_fim_debug_page(
//     postprocessed_messages: &[ContextFile],
//     // search_traces: &crate::ast::structs::AstCursorSearchResult,
// ) -> Value {
//     let mut context = json!({});
//     return context;
//     // XXX fix buckets
//     // fn shorter_symbol(x: &SymbolsSearchResultStruct) -> Value {
//     //     let mut t: Value = json!({});
//     //     t["name"] = Value::String(x.symbol_declaration.name.clone());
//     //     t["file_path"] = Value::String(x.symbol_declaration.file_path.display().to_string());
//     //     t["line1"] = json!(x.symbol_declaration.full_range.start_point.row + 1);
//     //     t["line2"] = json!(x.symbol_declaration.full_range.end_point.row + 1);
//     //     t
//     // }
//     // context["cursor_symbols"] = Value::Array(search_traces.cursor_symbols.iter()
//     //     .map(|x| shorter_symbol(x)).collect());
//     // context["bucket_declarations"] = Value::Array(search_traces.bucket_declarations.iter()
//     //     .map(|x| shorter_symbol(x)).collect());
//     // context["bucket_usage_of_same_stuff"] = Value::Array(search_traces.bucket_usage_of_same_stuff.iter()
//     //     .map(|x| shorter_symbol(x)).collect());
//     // context["bucket_high_overlap"] = Value::Array(search_traces.bucket_high_overlap.iter()
//     //     .map(|x| shorter_symbol(x)).collect());
//     // context["bucket_imports"] = Value::Array(search_traces.bucket_imports.iter()
//     //     .map(|x| shorter_symbol(x)).collect());

//     // let attached_files: Vec<_> = postprocessed_messages.iter().map(|x| {
//     //     json!({
//     //         "file_name": x.file_name,
//     //         "file_content": x.file_content,
//     //         "line1": x.line1,
//     //         "line2": x.line2,
//     //     })
//     // }).collect();
//     // context["attached_files"] = Value::Array(attached_files);
//     // context
// }
