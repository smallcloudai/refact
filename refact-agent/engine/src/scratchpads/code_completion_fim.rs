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
use crate::ast::ast_indexer_thread::AstIndexService;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{CodeCompletionPost, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::completion_cache;
use crate::scratchpad_abstract::{FinishReason, HasTokenizerAndEot, ScratchpadAbstract};
use crate::scratchpads::completon_rag::retrieve_ast_based_extra_context;
use crate::telemetry::snippets_collection;
use crate::telemetry::telemetry_structs;


const DEBUG: bool = false;

pub struct FillInTheMiddleScratchpad {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub order: String,
    pub fim_prefix: String,
    pub fim_suffix: String,
    pub fim_middle: String,
    pub extra_stop_tokens: Vec<String>,
    pub context_used: Value,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_service: Option<Arc<AMutex<AstIndexService>>>,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

impl FillInTheMiddleScratchpad {
    pub fn new(
        tokenizer: Option<Arc<Tokenizer>>,
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
            fim_suffix: String::new(),
            fim_middle: String::new(),
            extra_stop_tokens: vec![],
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

#[async_trait]
impl ScratchpadAbstract for FillInTheMiddleScratchpad {
    async fn apply_model_adaptation_patch(
        &mut self,
        patch: &Value,
    ) -> Result<(), String> {
        // That will work for some models (starcoder) without patching
        self.fim_prefix = patch.get("fim_prefix").and_then(|x| x.as_str()).unwrap_or("<fim_prefix>").to_string();
        self.fim_suffix = patch.get("fim_suffix").and_then(|x| x.as_str()).unwrap_or("<fim_suffix>").to_string();
        self.fim_middle = patch.get("fim_middle").and_then(|x| x.as_str()).unwrap_or("<fim_middle>").to_string();
        self.extra_stop_tokens = patch.get("extra_stop_tokens").map(|x| x.as_array().unwrap().into_iter().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<String>>()).unwrap_or(vec![]);
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.eos = patch.get("eos").and_then(|x| x.as_str()).unwrap_or("").to_string();
        self.t.context_format = patch.get("context_format").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        self.t.rag_ratio = patch.get("rag_ratio").and_then(|x| x.as_f64()).unwrap_or(0.5);
        if self.t.tokenizer.is_some() {
            self.t.assert_one_token(&self.fim_prefix.as_str())?;
            self.t.assert_one_token(&self.fim_suffix.as_str())?;
            self.t.assert_one_token(&self.fim_middle.as_str())?;
            self.t.assert_one_token(&self.t.eot.as_str())?;
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
            stop_list.extend(self.extra_stop_tokens.clone());
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
        self.context_used["fim_ms"] = Value::from(fim_ms);
        self.context_used["n_ctx".to_string()] = Value::from(n_ctx as i64);
        self.context_used["rag_tokens_limit".to_string()] = Value::from(rag_tokens_n as i64);
        info!(" -- /post fim {}ms-- ", fim_ms);


        if use_rag && rag_tokens_n > 0 {
            let pp_settings = {
                let ccx_locked = ccx.lock().await;
                ccx_locked.postprocess_parameters.clone()
            };

            // NOTE: why do we need this loop?
            // postprocess_context_files doesn't care about additional tokens after lines skip
            // in real world retrieve_ast_based_extra_context can produce context that doesn't fit in the budget
            // if so we need to reduce budget and retrieve context again
            let mut extra_content_collect_counter = 0;
            let mut content_tokens_budget = rag_tokens_n as i32;
            loop {
                let extra_context = retrieve_ast_based_extra_context(
                    self.global_context.clone(),
                    self.ast_service.clone(),
                    &self.t,
                    &cpath,
                    &pos,
                    (fim_line1, fim_line2),
                    pp_settings.clone(),
                    content_tokens_budget as usize,
                    &mut self.context_used
                ).await;
                let content_tokens_n = self.t.count_tokens(&extra_context.as_str())?;
                if content_tokens_n <= content_tokens_budget || extra_content_collect_counter > 1 {
                    prompt = format!("{extra_context}{prompt}");
                    break;
                } else {
                    content_tokens_budget -= content_tokens_n - content_tokens_budget;
                    extra_content_collect_counter += 1;
                }
            }
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
        finish_reasons: Vec<FinishReason>
    ) -> Result<Value, String> {
        let json_choices = choices.iter().enumerate().map(|(i, x)| {
            let cc = _cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline, &self.extra_stop_tokens);
            if i==0 {
                self.data4cache.completion0_text = cc.clone();
                self.data4cache.completion0_finish_reason = finish_reasons[i].to_string();
            }
            json!({
                "index": i,
                "code_completion": cc,
                "finish_reason": finish_reasons[i].to_json_val(),
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
        delta: String,
        finish_reason: FinishReason
    ) -> Result<(Value, FinishReason), String> {
        let json_choices= if !delta.is_empty() || finish_reason == FinishReason::Stop {
            let mut s: String = _cut_result(&delta, self.t.eot.as_str(), self.post.inputs.multiline, &self.extra_stop_tokens);
            if finish_reason.is_finished() {
                s = s.trim_end().to_string();
            }
            self.data4cache.completion0_text.push_str(&s);
            json!([{
                "index": 0,
                "code_completion": s,
                "finish_reason": finish_reason.to_json_val(),
            }])
        } else {
            assert_eq!(finish_reason, FinishReason::Length);
            json!([{
                "index": 0,
                "code_completion": "",
                "finish_reason": finish_reason.to_json_val()
            }])
        };
        self.data4cache.completion0_finish_reason = finish_reason.to_string();
        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache, self.context_used != json!({}));
        Ok((json!({
            "choices": json_choices,
            "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
        }), finish_reason))
    }

    fn response_message_streaming(
        &mut self,
        _delta: &Value,
        _finish_reason: FinishReason,
    ) -> Result<(Value, FinishReason), String> {
        Err("not implemented".to_string())
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        Err("".to_string())
    }

    fn streaming_finished(&mut self, finish_reason: FinishReason) -> Result<Value, String> {
        self.data4cache.completion0_finish_reason = finish_reason.to_string();
        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache, self.context_used != json!({}));
        Ok(json!({
            "choices": [{
                "index": 0,
                "code_completion": "",
                "finish_reason": finish_reason.to_json_val()
            }],
            "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
        }))
    }
}

fn _cut_result(text: &str, eot_token: &str, multiline: bool, extra_stop_tokens: &Vec<String>) -> String {
    let mut cut_at = vec![];
    if let Some(x) = text.find(eot_token) {
        cut_at.push(x);
    }
    for token in extra_stop_tokens {
        if let Some(x) = text.find(token) {
            cut_at.push(x);
        }
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
        return text.to_string().replace("\r", "");
    }
    let cut_at = cut_at.into_iter().min().unwrap_or(text.len());
    let ans = text.split_at(cut_at).0.to_string();
    ans.replace("\r", "")
}
