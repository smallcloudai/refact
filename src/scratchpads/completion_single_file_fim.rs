use std::path::PathBuf;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::CodeCompletionPost;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::vec;
use tokio::sync::Mutex as AMutex;
// use ropey::RopeSlice;
use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::info;
use async_trait::async_trait;
use serde_json::Value;
use tree_sitter::Point;
use crate::ast::ast_module::AstModule;

use crate::completion_cache;
use crate::files_in_workspace::DocumentInfo;
use crate::telemetry::telemetry_structs;
use crate::telemetry::snippets_collection;


const DEBUG: bool = false;


pub struct SingleFileFIM {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub order: String,
    pub fim_prefix: String,
    pub fim_suffix: String,
    pub fim_middle: String,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_module: Arc<AMutex<Option<AstModule>>>,
}

impl SingleFileFIM {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: CodeCompletionPost,
        order: String,
        cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
        tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
        ast_module: Arc<AMutex<Option<AstModule>>>,
    ) -> Self {
        let data4cache = completion_cache::CompletionSaveToCache::new(cache_arc, &post);
        let data4snippet = snippets_collection::SaveSnippet::new(tele_storage, &post);
        SingleFileFIM { t: HasTokenizerAndEot::new(tokenizer), post, order, fim_prefix: String::new(),
            fim_suffix: String::new(), fim_middle: String::new(), data4cache, data4snippet,
            ast_module
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
impl ScratchpadAbstract for SingleFileFIM {
    fn apply_model_adaptation_patch(
        &mut self,
        patch: &serde_json::Value,
    ) -> Result<(), String> {
        // That will work for some models (starcoder) without patching
        self.fim_prefix = patch.get("fim_prefix").and_then(|x| x.as_str()).unwrap_or("<fim_prefix>").to_string();
        self.fim_suffix = patch.get("fim_suffix").and_then(|x| x.as_str()).unwrap_or("<fim_suffix>").to_string();
        self.fim_middle = patch.get("fim_middle").and_then(|x| x.as_str()).unwrap_or("<fim_middle>").to_string();
        self.t.eot = patch.get("eot").and_then(|x| x.as_str()).unwrap_or("<|endoftext|>").to_string();
        self.t.eos = patch.get("eos").and_then(|x| x.as_str()).unwrap_or("").to_string();
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
        context_size: usize,
        sampling_parameters_to_patch: &mut SamplingParameters,
    ) -> Result<String, String> {
        let limit: i32 = context_size as i32 - self.post.parameters.max_new_tokens as i32;
        let supports_stop = true; // some hf models do not support stop, but it's a thing of the past?
        if supports_stop {
            let mut stop_list = vec![self.t.eot.clone(), "\n\n".to_string()];
            if !self.post.inputs.multiline {
                stop_list.push("\n".to_string());  // This doesn't stop hf inference, only whole tokens do
            }
            sampling_parameters_to_patch.stop = Some(stop_list);
        }
        let mut source = self.post.inputs.sources.get(
            &self.post.inputs.cursor.file)
            .ok_or("Cursor is in file not found in sources".to_string())?.clone();
        source = self.cleanup_prompt(&source);

        let text = Rope::from_str(&*source);

        let pos = &self.post.inputs.cursor;
        let file_path = PathBuf::from(self.post.inputs.cursor.file.clone());
        let mut before_iter = text.lines_at(pos.line as usize).reversed();
        let mut after_iter = text.lines_at(pos.line as usize + 1);
        let (extra_context, mut tokens_used) = match *self.ast_module.lock().await {
            Some(ref ast) => {
                ast_search(
                    ast,
                    &file_path,
                    &source,
                    Point { row: pos.line as usize, column: pos.character as usize },
                    self.t.clone(),
                    (limit as f32 * 0.5) as usize,
                ).await
            }
            None => (String::new(), 0)
        };

        let mut before_line = before_iter.next();

        let cursor_line1: String;
        let col = pos.character as usize;
        cursor_line1 = text.line(pos.line as usize).slice(0..col).to_string();
        // UNFINISHED LI|

        let mut after_line = after_iter.next();

        let cursor_line2: String;
        if self.post.inputs.multiline {
            cursor_line2 = text.line(pos.line as usize).slice(col..).to_string();
        } else {
            cursor_line2 = "".to_string();
        }

        let mut before = vec![];
        let mut after = String::new();
        tokens_used += self.t.count_tokens(
            (cursor_line1.clone() + &cursor_line2).as_str()
        )?;
        while before_line.is_some() || after_line.is_some() {
            if let Some(before_line) = before_line {
                let before_line = before_line.to_string();
                let tokens = self.t.count_tokens(before_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                before.push(before_line);
            }
            if let Some(after_line) = after_line {
                let after_line = after_line.to_string();
                let tokens = self.t.count_tokens(after_line.as_str())?;
                if tokens_used + tokens > limit {
                    break;
                }
                tokens_used += tokens;
                after.push_str(&after_line);
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }
        info!("single file FIM prompt {} tokens used < limit {}", tokens_used, limit);
        let prompt: String;
        if self.order == "PSM" {
            prompt = format!(
                "{}{}{}{}{}{}{}{}{}",
                self.t.eos,
                self.fim_prefix,
                extra_context,
                before.into_iter().rev().collect::<Vec<_>>().join(""),
                cursor_line1,
                self.fim_suffix,
                cursor_line2,
                after,
                self.fim_middle
            );
        } else if self.order == "SPM" {
            prompt = format!(
                "{}{}{}{}{}{}{}{}{}",
                self.t.eos,
                self.fim_suffix,
                extra_context,
                cursor_line2,
                after,
                self.fim_prefix,
                before.into_iter().rev().collect::<Vec<_>>().join(""),
                cursor_line1,
                self.fim_middle,
            );
        } else {
            return Err(format!("order \"{}\" not recognized", self.order));
        }
        if DEBUG {
            info!("cursor position\n{:?}", self.post.inputs.cursor);
            info!("prompt\n{}", prompt);
            info!("re-encode whole prompt again gives {} tokens", self.t.count_tokens(prompt.as_str())?);
        }
        Ok(prompt)
    }

    fn response_n_choices(
        &mut self,
        choices: Vec<String>,
        stopped: Vec<bool>
    ) -> Result<serde_json::Value, String> {
        let json_choices = choices.iter().enumerate().map(|(i, x)| {
            let (mut cc, mut finished) = cut_result(&x, self.t.eot.as_str(), self.post.inputs.multiline);
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
            serde_json::json!({
                "index": i,
                "code_completion": cc,
                "finish_reason": finish_reason.clone(),
            })
        }).collect::<Vec<_>>();

        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache);
        return Ok(serde_json::json!(
            {
                "choices": json_choices,
                "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
                "model": self.post.model.clone(),
            }
        ));
    }

    fn response_streaming(
        &mut self,
        delta: String,
        stop_toks: bool,
        stop_length: bool,
    ) -> Result<(serde_json::Value, bool), String> {
        let mut finished;
        let json_choices;
        // info!("XXXXX delta: {:?}", delta);
        // info!("XXXXX stop_toks: {:?}", stop_toks);
        // info!("XXXXX stop_length: {:?}", stop_length);
        if !delta.is_empty() || stop_toks {
            let mut s: String;
            (s, finished) = cut_result(&delta, self.t.eot.as_str(), self.post.inputs.multiline);
            finished |= stop_toks;
            if finished {
                // can stay consistent with trim() only if that's the final iteration
                s = s.trim_end().to_string();
                self.data4cache.completion0_finish_reason = if finished { "stop".to_string() } else { "".to_string() };
            }
            self.data4cache.completion0_text.push_str(&s);
            json_choices = serde_json::json!([{
                "index": 0,
                "code_completion": s,
                "finish_reason": if finished { serde_json::Value::String("stop".to_string()) } else { serde_json::Value::Null },
            }]);
        } else {
            assert!(stop_length);
            json_choices = serde_json::json!([{
                "index": 0,
                "code_completion": "",
                "finish_reason": "length"
            }]);
            self.data4cache.completion0_finish_reason = "length".to_string();
            finished = true;
        }
        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache);
        let ans = serde_json::json!({
            "choices": json_choices,
            "snippet_telemetry_id": self.data4cache.completion0_snippet_telemetry_id,
        });
        Ok((ans, finished))
    }

    fn response_spontaneous(&mut self) -> Result<Vec<Value>, String>  {
        return Err("".to_string());
    }
}

async fn ast_search(
    ast_module: &AstModule,
    file_path: &PathBuf,
    code: &str,
    cursor: Point,
    tokenizer: HasTokenizerAndEot,
    max_context_size: usize
) -> (String, i32){
    let doc = match DocumentInfo::from(file_path).ok() {
        Some(doc) => doc,
        None => return ("".to_string(), 0)
    };
    let search_result = ast_module.search_by_cursor(
        &doc, code, cursor, 5
    ).await;

    let init_cfc_text = "Here are some relevant code fragments from other files of the repo:\n\n";
    let mut tokens_used = tokenizer.count_tokens(init_cfc_text).expect(
        "Tokenization has failed"
    );
    match search_result {
        Ok(res) => {
            if res.search_results.is_empty() {
                return ("".to_string(), tokens_used);
            }

            let mut final_text_vec: Vec<String> = vec![init_cfc_text.to_string()];
            for res in res.search_results {
                let text: String = format!(
                    "The below code fragment is found in {}\n{}\n\n",
                    res.symbol_declaration.meta_path,
                    res.symbol_declaration.get_content().await.unwrap_or("".to_string())
                );
                tokens_used += tokenizer.count_tokens(&text).expect(
                    "Tokenization has failed"
                );
                final_text_vec.push(text);
                if tokens_used > max_context_size as i32 {
                    break
                }
            }
            (final_text_vec.join(""), tokens_used)
        }
        Err(_) => ("".to_string(), tokens_used)
    }
}


fn cut_result(text: &str, eot_token: &str, multiline: bool) -> (String, bool) {
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

