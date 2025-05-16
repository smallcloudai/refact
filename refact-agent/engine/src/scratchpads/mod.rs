use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

pub mod code_completion_fim;
pub mod chat_generic;
pub mod chat_passthrough;
pub mod chat_utils_deltadelta;
pub mod chat_utils_limit_history;
pub mod chat_utils_prompts;
pub mod token_count_cache;
pub mod scratchpad_utils;
pub mod code_completion_replace;
pub mod multimodality;
mod comments_parser;
mod passthrough_convert_messages;
mod completon_rag;

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::call_validation::{ChatMessage, CodeCompletionPost};
use crate::call_validation::ChatPost;
use crate::caps::ChatModelRecord;
use crate::caps::CompletionModelRecord;
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::completion_cache;
use crate::telemetry::telemetry_structs;
use crate::tokens;
use crate::tools::tools_description::ToolDesc;


fn verify_has_send<T: Send>(_x: &T) {}


pub async fn create_code_completion_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    model_rec: &CompletionModelRecord,
    post: &CodeCompletionPost,
    cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
    tele_storage: Arc<StdRwLock<telemetry_structs::Storage>>,
    ast_module: Option<Arc<AMutex<AstIndexService>>>,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    let tokenizer_arc = crate::tokens::cached_tokenizer(global_context.clone(), &model_rec.base).await?;
    if model_rec.scratchpad == "FIM-PSM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "PSM".to_string(), cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "FIM-SPM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "SPM".to_string(), cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "REPLACE" {
        result = Box::new(code_completion_replace::CodeCompletionReplaceScratchpad::new(
            tokenizer_arc, &post, cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "REPLACE_PASSTHROUGH" {
        result = Box::new(code_completion_replace::CodeCompletionReplacePassthroughScratchpad::new(
            tokenizer_arc, &post, cache_arc, tele_storage, ast_module, global_context.clone()
        ))
    } else {
        return Err(format!("This rust binary doesn't have code completion scratchpad \"{}\" compiled in", model_rec.scratchpad));
    }
    result.apply_model_adaptation_patch(&model_rec.scratchpad_patch).await?;
    verify_has_send(&result);
    Ok(result)
}

pub async fn create_chat_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    post: &mut ChatPost,
    tools: Vec<ToolDesc>,
    messages: &Vec<ChatMessage>,
    prepend_system_prompt: bool,
    model_rec: &ChatModelRecord,
    allow_at: bool,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    let tokenizer_arc = tokens::cached_tokenizer(global_context.clone(), &model_rec.base).await?;
    if model_rec.scratchpad == "CHAT-GENERIC" {
        result = Box::new(chat_generic::GenericChatScratchpad::new(
            tokenizer_arc.clone(), post, messages, prepend_system_prompt, allow_at
        ));
    } else if model_rec.scratchpad == "PASSTHROUGH" {
        result = Box::new(chat_passthrough::ChatPassthrough::new(
            tokenizer_arc.clone(), 
            post, 
            tools,
            messages, 
            prepend_system_prompt, 
            allow_at, 
            model_rec.supports_tools, 
            model_rec.supports_clicks,
        ));
    } else {
        return Err(format!("This rust binary doesn't have chat scratchpad \"{}\" compiled in", model_rec.scratchpad));
    }
    result.apply_model_adaptation_patch(&model_rec.scratchpad_patch).await?;
    verify_has_send(&result);
    Ok(result)
}
