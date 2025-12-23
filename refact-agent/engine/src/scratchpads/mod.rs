use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

pub mod code_completion_fim;
pub mod chat_utils_prompts;
pub mod scratchpad_utils;
pub mod code_completion_replace;
pub mod multimodality;
mod comments_parser;
mod completon_rag;
pub mod system_context;
#[cfg(test)]
mod chat_utils_limit_history_tests;

use crate::ast::ast_indexer_thread::AstIndexService;
use crate::call_validation::CodeCompletionPost;
use crate::caps::CompletionModelRecord;
use crate::completion_cache;
use crate::global_context::GlobalContext;
use crate::scratchpad_abstract::ScratchpadAbstract;


fn verify_has_send<T: Send>(_x: &T) {}


pub async fn create_code_completion_scratchpad(
    global_context: Arc<ARwLock<GlobalContext>>,
    model_rec: &CompletionModelRecord,
    post: &CodeCompletionPost,
    cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
    ast_module: Option<Arc<AMutex<AstIndexService>>>,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    let tokenizer_arc = crate::tokens::cached_tokenizer(global_context.clone(), &model_rec.base).await?;
    if model_rec.scratchpad == "FIM-PSM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "PSM".to_string(), cache_arc, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "FIM-SPM" {
        result = Box::new(code_completion_fim::FillInTheMiddleScratchpad::new(
            tokenizer_arc, &post, "SPM".to_string(), cache_arc, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "REPLACE" {
        result = Box::new(code_completion_replace::CodeCompletionReplaceScratchpad::new(
            tokenizer_arc, &post, cache_arc, ast_module, global_context.clone()
        ))
    } else if model_rec.scratchpad == "REPLACE_PASSTHROUGH" {
        result = Box::new(code_completion_replace::CodeCompletionReplacePassthroughScratchpad::new(
            tokenizer_arc, &post, cache_arc, ast_module, global_context.clone()
        ))
    } else {
        return Err(format!("This rust binary doesn't have code completion scratchpad \"{}\" compiled in", model_rec.scratchpad));
    }
    result.apply_model_adaptation_patch(&model_rec.scratchpad_patch).await?;
    verify_has_send(&result);
    Ok(result)
}
