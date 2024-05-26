use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_ast_definition::AtAstDefinition;
use crate::at_commands::at_ast_lookup_symbols::AtAstLookupSymbols;
use crate::at_commands::at_ast_reference::AtAstReference;
use crate::at_commands::at_file::AtFile;
use crate::at_commands::at_workspace::AtWorkspace;
use crate::at_commands::at_local_notes_to_self::AtLocalNotesToSelf;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::global_context::GlobalContext;


pub struct AtCommandsContext {
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub at_commands: HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>>,
    pub top_n: usize,
}

impl AtCommandsContext {
    pub async fn new(global_context: Arc<ARwLock<GlobalContext>>, top_n: usize) -> Self {
        AtCommandsContext {
            global_context,
            at_commands: at_commands_dict().await,
            top_n,
        }
    }
}

#[async_trait]
pub trait AtCommand: Send + Sync {
    fn name(&self) -> &String;
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>;

    fn works_as_tool(&self) -> bool { false }
    fn works_as_at_command(&self) -> bool { true }

    // returns (messages_for_postprocessing, text_on_clip)
    async fn execute_as_at_command(&self, _ccx: &mut AtCommandsContext, _query: &String, _args: &Vec<String>) -> Result<(Vec<ContextEnum>, String), String> {
        unimplemented!();
    }

    async fn execute_as_tool(&self, _ccx: &mut AtCommandsContext, _tool_call_id: &String, _args: &HashMap<String, serde_json::Value>) -> Result<Vec<ContextEnum>, String> {
        unimplemented!();
    }

    fn depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"
}

#[async_trait]
pub trait AtParam: Send + Sync {
    fn name(&self) -> &String;
    async fn is_value_valid(&self, value: &String, ccx: &AtCommandsContext) -> bool;
    async fn complete(&self, value: &String, ccx: &AtCommandsContext) -> Vec<String>;
    fn complete_if_valid(&self) -> bool {false}
}

pub struct AtCommandCall {
    pub command: Arc<AMutex<Box<dyn AtCommand + Send>>>,
    pub args: Vec<String>,
}

impl AtCommandCall {
    pub fn new(command: Arc<AMutex<Box<dyn AtCommand + Send>>>, args: Vec<String>) -> Self {
        AtCommandCall {
            command,
            args,
        }
    }
}

pub async fn at_commands_dict() -> HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>> {
    return HashMap::from([
        ("@workspace".to_string(), Arc::new(AMutex::new(Box::new(AtWorkspace::new()) as Box<dyn AtCommand + Send>))),
        ("@file".to_string(), Arc::new(AMutex::new(Box::new(AtFile::new()) as Box<dyn AtCommand + Send>))),
        ("@definition".to_string(), Arc::new(AMutex::new(Box::new(AtAstDefinition::new()) as Box<dyn AtCommand + Send>))),
        ("@references".to_string(), Arc::new(AMutex::new(Box::new(AtAstReference::new()) as Box<dyn AtCommand + Send>))),
        ("@symbols-at".to_string(), Arc::new(AMutex::new(Box::new(AtAstLookupSymbols::new()) as Box<dyn AtCommand + Send>))),
        ("@local-notes-to-self".to_string(), Arc::new(AMutex::new(Box::new(AtLocalNotesToSelf::new()) as Box<dyn AtCommand + Send>))),
    ]);
}

pub fn vec_context_file_to_context_tools(x: Vec<ContextFile>) -> Vec<ContextEnum> {
    x.into_iter().map(|i|ContextEnum::ContextFile(i)).collect::<Vec<ContextEnum>>()
}

// pub fn vec_chat_msg_into_tools(x: Vec<ChatMessage>) -> Vec<ContextEnum> {
//     x.into_iter().map(|i|ContextEnum::ChatMessage(i)).collect::<Vec<ContextEnum>>()
// }

pub fn filter_only_context_file_from_context_tool(tools: &Vec<ContextEnum>) -> Vec<ContextFile> {
    tools.iter()
        .filter_map(|x| {
            if let ContextEnum::ContextFile(data) = x { Some(data.clone()) } else { None }
        }).collect::<Vec<ContextFile>>()
}

// pub fn filter_chat_msg_from_tools(tools: &Vec<ContextEnum>) -> Vec<ChatMessage> {
//     tools.iter()
//         .filter_map(|x| {
//             if let ContextEnum::ChatMessage(data) = x { Some(data.clone()) } else { None }
//         }).collect::<Vec<ChatMessage>>()
// }
