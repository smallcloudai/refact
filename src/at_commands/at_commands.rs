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
use crate::call_validation::ChatMessage;
use crate::global_context::GlobalContext;

pub struct AtCommandsContext {
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub at_commands: HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>>,
}

impl AtCommandsContext {
    pub async fn new(global_context: Arc<ARwLock<GlobalContext>>) -> Self {
        AtCommandsContext {
            global_context,
            at_commands: at_commands_dict().await,
        }
    }
}

#[async_trait]
pub trait AtCommand: Send + Sync {
    fn name(&self) -> &String;
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>;
    async fn can_execute(&self, _args: &Vec<String>, _context: &AtCommandsContext) -> bool {true}
    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String>;
}

#[async_trait]
pub trait AtParam: Send + Sync {
    fn name(&self) -> &String;
    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool;
    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String>;
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
    ]);
}
