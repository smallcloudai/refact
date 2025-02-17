use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokenizers::Tokenizer;

pub mod code_completion_fim;
pub mod chat_generic;
pub mod chat_llama2;
pub mod chat_passthrough;
pub mod chat_utils_deltadelta;
pub mod chat_utils_limit_history;
pub mod chat_utils_prompts;
pub mod scratchpad_utils;
pub mod code_completion_replace;
pub mod multimodality;
mod comments_parser;
mod passthrough_convert_messages;
mod completon_rag;

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::call_validation::{ChatMessage, CodeCompletionPost};
use crate::call_validation::ChatPost;
use crate::global_context::GlobalContext;
use crate::caps::CodeAssistantCaps;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::completion_cache;
use crate::telemetry::telemetry_structs;
use crate::cached_tokenizers;


fn verify_has_send<T: Send>(_x: &T) {}


pub async fn create_code_completion_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    model_name_for_tokenizer: String,
    post: &CodeCompletionPost,
    scratchpad_name: &str,
    scratchpad_patch: &serde_json::Value,
    cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
    tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
    ast_module: Option<Arc<AMutex<AstIndexService>>>,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    let tokenizer_arc: Arc<StdRwLock<Tokenizer>> = cached_tokenizers::cached_tokenizer(caps, global_context.clone(), model_name_for_tokenizer).await?;
    if scratchpad_name == "FIM-PSM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "PSM".to_string(), cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if scratchpad_name == "FIM-SPM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "SPM".to_string(), cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if scratchpad_name == "REPLACE" {
        result = Box::new(code_completion_replace::CodeCompletionReplaceScratchpad::new(
            tokenizer_arc, &post, cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if scratchpad_name == "REPLACE_PASSTHROUGH" {
        result = Box::new(code_completion_replace::CodeCompletionReplacePassthroughScratchpad::new(
            tokenizer_arc, &post, cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else {
        return Err(format!("This rust binary doesn't have code completion scratchpad \"{}\" compiled in", scratchpad_name));
    }
    result.apply_model_adaptation_patch(scratchpad_patch, false, false).await?;
    verify_has_send(&result);
    Ok(result)
}

pub async fn create_chat_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    caps: Arc<StdRwLock<CodeAssistantCaps>>,
    model_name_for_tokenizer: String,
    post: &mut ChatPost,
    messages: &Vec<ChatMessage>,
    prepend_system_prompt: bool,
    scratchpad_name: &str,
    scratchpad_patch: &serde_json::Value,
    allow_at: bool,
    supports_tools: bool,
    supports_clicks: bool,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    let tokenizer_arc = cached_tokenizers::cached_tokenizer(caps, global_context.clone(), model_name_for_tokenizer).await?;
    if scratchpad_name == "CHAT-GENERIC" {
        result = Box::new(chat_generic::GenericChatScratchpad::new(
            tokenizer_arc.clone(), post, messages, allow_at
        ));
    } else if scratchpad_name == "CHAT-LLAMA2" {
        result = Box::new(chat_llama2::ChatLlama2::new(
            tokenizer_arc.clone(), post, messages, allow_at
        ));
    } else if scratchpad_name == "PASSTHROUGH" {
        result = Box::new(chat_passthrough::ChatPassthrough::new(
            tokenizer_arc.clone(), post, messages, prepend_system_prompt, allow_at, supports_tools, supports_clicks
        ));
    } else {
        return Err(format!("This rust binary doesn't have chat scratchpad \"{}\" compiled in", scratchpad_name));
    }
    let mut exploration_tools: bool = false;
    let mut agentic_tools: bool = false;
    if post.tools.is_some() {
        for t in post.tools.as_ref().unwrap() {
            let tobj = t.as_object().unwrap();
            if let Some(function) = tobj.get("function") {
                if let Some(name) = function.get("name") {
                    if name.as_str() == Some("web") {  // anything that will still be on without ast and vecdb
                        exploration_tools = true;
                    }
                    if name.as_str() == Some("apply_edit") {
                        agentic_tools = true;
                    }
                }
            }
        }
    }
    result.apply_model_adaptation_patch(scratchpad_patch, exploration_tools, agentic_tools).await?;
    verify_has_send(&result);
    Ok(result)
}
