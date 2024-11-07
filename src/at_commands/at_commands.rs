use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, ContextFile, ContextEnum, SubchatParameters, PostprocessSettings};
use crate::global_context::GlobalContext;

use crate::at_commands::at_file::AtFile;
use crate::at_commands::at_ast_definition::AtAstDefinition;
use crate::at_commands::at_ast_reference::AtAstReference;
use crate::at_commands::at_tree::AtTree;
use crate::at_commands::at_web::AtWeb;
use crate::at_commands::execute_at::AtCommandMember;


pub struct AtCommandsContext {
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub n_ctx: usize,
    pub top_n: usize,
    pub tokens_for_rag: usize,
    pub messages: Vec<ChatMessage>,
    #[allow(dead_code)]
    pub is_preview: bool,
    pub pp_skeleton: bool,
    pub correction_only_up_to_step: usize,  // suppresses context_file messages, writes a correction message instead
    pub chat_id: String,
    pub should_execute_remotely: bool,

    pub at_commands: HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>>,  // a copy from static constant
    pub subchat_tool_parameters: IndexMap<String, SubchatParameters>,
    pub postprocess_parameters: PostprocessSettings,

    pub subchat_tx: Arc<AMutex<mpsc::UnboundedSender<serde_json::Value>>>, // one and only supported format for now {"tool_call_id": xx, "subchat_id": xx, "add_message": {...}}
    pub subchat_rx: Arc<AMutex<mpsc::UnboundedReceiver<serde_json::Value>>>,
}

impl AtCommandsContext {
    pub async fn new(
        global_context: Arc<ARwLock<GlobalContext>>,
        n_ctx: usize,
        top_n: usize,
        is_preview: bool,
        messages: Vec<ChatMessage>,
        chat_id: String,
        should_execute_remotely: bool,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<serde_json::Value>();
        AtCommandsContext {
            global_context: global_context.clone(),
            n_ctx,
            top_n,
            tokens_for_rag: 0,
            messages,
            is_preview,
            pp_skeleton: false,
            correction_only_up_to_step: 0,
            chat_id,
            should_execute_remotely,

            at_commands: at_commands_dict(global_context.clone()).await,
            subchat_tool_parameters: IndexMap::new(),
            postprocess_parameters: PostprocessSettings::new(),

            subchat_tx: Arc::new(AMutex::new(tx)),
            subchat_rx: Arc::new(AMutex::new(rx)),
        }
    }
}

#[async_trait]
pub trait AtCommand: Send + Sync {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>;
    // returns (messages_for_postprocessing, text_on_clip)
    async fn at_execute(&self, ccx: Arc<AMutex<AtCommandsContext>>, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String>;
    fn depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"
}

#[async_trait]
pub trait AtParam: Send + Sync {
    async fn is_value_valid(&self, ccx: Arc<AMutex<AtCommandsContext>>, value: &String) -> bool;
    async fn param_completion(&self, ccx: Arc<AMutex<AtCommandsContext>>, value: &String) -> Vec<String>;
    fn param_completion_valid(&self) -> bool {false}
}

pub async fn at_commands_dict(gcx: Arc<ARwLock<GlobalContext>>) -> HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>> {
    let at_commands_dict = HashMap::from([
        ("@file".to_string(), Arc::new(AMutex::new(Box::new(AtFile::new()) as Box<dyn AtCommand + Send>))),
        // ("@file-search".to_string(), Arc::new(AMutex::new(Box::new(AtFileSearch::new()) as Box<dyn AtCommand + Send>))),
        ("@definition".to_string(), Arc::new(AMutex::new(Box::new(AtAstDefinition::new()) as Box<dyn AtCommand + Send>))),
        ("@references".to_string(), Arc::new(AMutex::new(Box::new(AtAstReference::new()) as Box<dyn AtCommand + Send>))),
        // ("@local-notes-to-self".to_string(), Arc::new(AMutex::new(Box::new(AtLocalNotesToSelf::new()) as Box<dyn AtCommand + Send>))),
        ("@tree".to_string(), Arc::new(AMutex::new(Box::new(AtTree::new()) as Box<dyn AtCommand + Send>))),
        // ("@diff".to_string(), Arc::new(AMutex::new(Box::new(AtDiff::new()) as Box<dyn AtCommand + Send>))),
        // ("@diff-rev".to_string(), Arc::new(AMutex::new(Box::new(AtDiffRev::new()) as Box<dyn AtCommand + Send>))),
        ("@web".to_string(), Arc::new(AMutex::new(Box::new(AtWeb::new()) as Box<dyn AtCommand + Send>))),
        #[cfg(feature="vecdb")]
        ("@search".to_string(), Arc::new(AMutex::new(Box::new(crate::at_commands::at_search::AtSearch::new()) as Box<dyn AtCommand + Send>))),
    ]);

    let (ast_on, vecdb_on) = {
        let gcx_locked = gcx.read().await;
        #[cfg(feature="vecdb")]
        let vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        #[cfg(not(feature="vecdb"))]
        let vecdb_on = false;
        (gcx_locked.ast_service.is_some(), vecdb_on)
    };

    let mut result = HashMap::new();
    for (key, value) in at_commands_dict {
        let command = value.lock().await;
        let depends_on = command.depends_on();
        if depends_on.contains(&"ast".to_string()) && !ast_on {
            continue;
        }
        if depends_on.contains(&"vecdb".to_string()) && !vecdb_on {
            continue;
        }
        result.insert(key, value.clone());
    }

    result
}

pub fn vec_context_file_to_context_tools(x: Vec<ContextFile>) -> Vec<ContextEnum> {
    x.into_iter().map(|i|ContextEnum::ContextFile(i)).collect::<Vec<ContextEnum>>()
}

pub fn filter_only_context_file_from_context_tool(tools: &Vec<ContextEnum>) -> Vec<ContextFile> {
    tools.iter()
        .filter_map(|x| {
            if let ContextEnum::ContextFile(data) = x { Some(data.clone()) } else { None }
        }).collect::<Vec<ContextFile>>()
}

