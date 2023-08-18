use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::Mutex as AMutex;

use tokenizers::Tokenizer;

pub mod completion_single_file_fim;
pub mod chat_generic;
pub mod chat_llama2;
pub mod chat_utils_deltadelta;
pub mod chat_utils_limit_history;

use crate::call_validation::CodeCompletionPost;
use crate::call_validation::ChatPost;
use crate::scratchpad_abstract::ScratchpadAbstract;
use crate::completion_cache;
use crate::telemetry_storage;
use crate::vecdb_search;


fn verify_has_send<T: Send>(_x: &T) {}


pub fn create_code_completion_scratchpad(
    post: CodeCompletionPost,
    scratchpad_name: &str,
    scratchpad_patch: &serde_json::Value,
    tokenizer_arc: Arc<StdRwLock<Tokenizer>>,
    cache_arc: Arc<StdRwLock<completion_cache::CompletionCache>>,
    tele_storage: Arc<StdRwLock<telemetry_storage::Storage>>,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    if scratchpad_name == "FIM-PSM" {
        result = Box::new(completion_single_file_fim::SingleFileFIM::new(tokenizer_arc, post, "PSM".to_string(), cache_arc, tele_storage));
    } else if scratchpad_name == "FIM-SPM" {
        result = Box::new(completion_single_file_fim::SingleFileFIM::new(tokenizer_arc, post, "SPM".to_string(), cache_arc, tele_storage));
    } else {
        return Err(format!("This rust binary doesn't have code completion scratchpad \"{}\" compiled in", scratchpad_name));
    }
    result.apply_model_adaptation_patch(scratchpad_patch)?;
    verify_has_send(&result);
    Ok(result)
}

pub fn create_chat_scratchpad(
    post: ChatPost,
    scratchpad_name: &str,
    scratchpad_patch: &serde_json::Value,
    tokenizer_arc: Arc<StdRwLock<Tokenizer>>,
    vecdb_search: Arc<AMutex<Box<dyn vecdb_search::VecdbSearch + Send>>>,
) -> Result<Box<dyn ScratchpadAbstract>, String> {
    let mut result: Box<dyn ScratchpadAbstract>;
    if scratchpad_name == "CHAT-GENERIC" {
        result = Box::new(chat_generic::GenericChatScratchpad::new(tokenizer_arc, post, vecdb_search));
    } else if scratchpad_name == "CHAT-LLAMA2" {
        result = Box::new(chat_llama2::ChatLlama2::new(tokenizer_arc, post, vecdb_search));
    } else {
        return Err(format!("This rust binary doesn't have chat scratchpad \"{}\" compiled in", scratchpad_name));
    }
    result.apply_model_adaptation_patch(scratchpad_patch)?;
    verify_has_send(&result);
    Ok(result)
}
