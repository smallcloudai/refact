use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::vec;

use async_trait::async_trait;
use ropey::Rope;
use serde_json::{Value, json};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use tracing::{info, error};
use tree_sitter::Point;

use crate::ast::ast_module::AstModule;
// use crate::ast::comments_wrapper::{get_language_id_by_filename, wrap_comments};
use crate::call_validation::{CodeCompletionPost, SamplingParameters};
use crate::global_context::GlobalContext;
use crate::completion_cache;
use crate::files_in_workspace::DocumentInfo;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::telemetry::snippets_collection;
use crate::telemetry::telemetry_structs;


const DEBUG: bool = false;


pub struct SingleFileFIM {
    pub t: HasTokenizerAndEot,
    pub post: CodeCompletionPost,
    pub order: String,
    pub fim_prefix: String,
    pub fim_suffix: String,
    pub fim_middle: String,
    pub context_used: serde_json::Value,
    pub data4cache: completion_cache::CompletionSaveToCache,
    pub data4snippet: snippets_collection::SaveSnippet,
    pub ast_module: Option<Arc<ARwLock<AstModule>>>,
    pub global_context: Arc<ARwLock<GlobalContext>>,
}

impl SingleFileFIM {
    pub fn new(
        tokenizer: Arc<StdRwLock<Tokenizer>>,
        post: CodeCompletionPost,
        order: String,
        cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
        tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
        ast_module: Option<Arc<ARwLock<AstModule>>>,
        global_context: Arc<ARwLock<GlobalContext>>,
    ) -> Self {
        let data4cache = completion_cache::CompletionSaveToCache::new(cache_arc, &post);
        let data4snippet = snippets_collection::SaveSnippet::new(tele_storage, &post);
        SingleFileFIM { t: HasTokenizerAndEot::new(tokenizer), post, order, fim_prefix: String::new(),
            fim_suffix: String::new(), fim_middle: String::new(),
            context_used: json!([]),
            data4cache,
            data4snippet,
            ast_module,
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
                &self.post.inputs.cursor.file
            ).ok_or("Cursor is in file not found in sources".to_string())?.clone();
        source = self.cleanup_prompt(&source);

        let text = Rope::from_str(&*source);

        let pos = &self.post.inputs.cursor;
        let file_path = PathBuf::from(self.post.inputs.cursor.file.clone());
        let mut before_iter = text.lines_at(pos.line as usize).reversed();
        let mut after_iter = text.lines_at(pos.line as usize + 1);
        let extra_context = String::new();
        let mut tokens_used = 0;
        let ast_messages: Vec<crate::call_validation::ChatMessage> = if true /*self.post.use_ast*/ {
            let chat_message_maybe = match &self.ast_module {
                Some(ast) => {
                    let doc_info = match DocumentInfo::from_pathbuf(&file_path) {
                        Ok(doc) => doc,
                        Err(err) => {
                            error!("can't get doc info for {}: {}", file_path.display(), err);
                            return Err(err.to_string());
                        }
                    };
                    match ast.read().await.retrieve_cursor_symbols_by_declarations(
                        &doc_info, &source, Point { row: pos.line as usize, column: pos.character as usize },
                        5, 5
                    ).await {
                        Ok(res) => Ok(crate::at_commands::at_ast_lookup_symbols::results2message(&res).await),
                        Err(err) => Err(err)
                    }
                }
                None => { Err("ast not initialized".to_string()) }
            };
            match chat_message_maybe {
                Ok(context_message) => { vec![context_message] }
                Err(err) => { error!("can't fetch ast results: {}", err); vec![] }
            }
        } else {
            vec![]
        };
        let context_ctx_this_message = 1024;  // FIXME: calculate better, subtract from limit
        let postprocessed_messages = crate::scratchpads::chat_utils_rag::postprocess_at_results2(
            self.global_context.clone(),
            ast_messages,
            self.t.tokenizer.clone(),
            context_ctx_this_message,
        ).await;
        self.context_used = json!(postprocessed_messages);

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
        if DEBUG {
            info!("response_n_choices\n{:?}", json_choices);
        }

        snippets_collection::snippet_register_from_data4cache(&self.data4snippet, &mut self.data4cache);
        return Ok(serde_json::json!(
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

// async fn ast_search(
//     ast_module: &mut AstModule,
//     file_path: &PathBuf,
//     code: &str,
//     cursor: Point,
//     // tokenizer: HasTokenizerAndEot,
//     max_context_size: usize
// ) -> (String, i32)
// {
//     let doc = match DocumentInfo::from_pathbuf(file_path).ok() {
//         Some(doc) => doc,
//         None => return ("".to_string(), 0)
//     };
//     let lang = get_language_id_by_filename(&doc.get_path()).unwrap_or_default();
//     let declarations_str = wrap_comments("Useful declarations:\n ", &lang);
//     let references_str = wrap_comments("Useful references:\n ", &lang);

//     let search_result = ast_module.search_declarations_by_cursor(
//         &doc, code, cursor, 5, true
//     ).await;
//     let mut extra_context: Vec<String> = vec!();


    // let mut tokens_used = tokenizer.count_tokens(&declarations_str).expect(
    //     "Tokenization has failed"
    // );

    // match search_result {
    //     Ok(res) => {
    //         if res.search_results.len() > 0 {
    //             extra_context.push(declarations_str.to_string());
    //         }
    //         for res in res.search_results {
    //             let code: String = format!(
    //                 "symbol path: {}\ncode: \n{}\n\n",
    //                 res.symbol_declaration.meta_path,
    //                 res.symbol_declaration.get_content().await.unwrap_or(" ".to_string())
    //             );
    //             let text = wrap_comments(&code, &lang);
    //             let tokens = tokenizer.count_tokens(&text).expect(
    //                 "Tokenization has failed"
    //             );
    //             if (tokens_used + tokens) > (max_context_size / 2) as i32 {
    //                 break
    //             } else {
    //                 extra_context.push(text);
    //                 tokens_used += tokens;
    //             }
    //         }
    //     }
    //     Err(err) => {
    //         info!("Error while calling `search_declarations_by_cursor` {:?}", err);
    //     }
    // }

    // let search_result = ast_module.search_references_by_cursor(
    //     &doc, code, cursor, 5, true
    // ).await;
    // let mut tokens_used = tokenizer.count_tokens(&references_str).expect(
    //     "Tokenization has failed"
    // );
    // match search_result {
    //     Ok(res) => {
    //         if res.search_results.len() > 0 {
    //             extra_context.push(references_str.to_string());
    //         }
    //         for res in res.search_results {
    //             let code: String = format!(
    //                 "symbol path: {}\ncode: {}\n\n",
    //                 res.symbol_declaration.meta_path,
    //                 res.symbol_declaration.get_content().await.unwrap_or("".to_string())
    //             );
    //             let text = wrap_comments(&code, &lang);
    //             let tokens = tokenizer.count_tokens(&text).expect(
    //                 "Tokenization has failed"
    //             );
    //             if (tokens_used + tokens) > (max_context_size / 2) as i32 {
    //                 break
    //             } else {
    //                 extra_context.push(text);
    //                 tokens_used += tokens;
    //             }
    //         }
    //     }
    //     Err(err) => {
    //         info!("Error while calling `search_declarations_by_cursor` {:?}", err);
    //     }
    // }

//     (extra_context.join(""), tokens_used)
// }

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

