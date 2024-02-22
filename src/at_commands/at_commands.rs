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
    async fn are_args_valid(&self, args: &Vec<String>, context: &AtCommandsContext) -> Vec<bool>;
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool;
    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String>;
}

#[async_trait]
pub trait AtParam: Send + Sync {
    fn name(&self) -> &String;
    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool;
    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String>;
    fn complete_if_valid(&self) -> bool;
}

pub struct AtCommandCall {
    pub command: Arc<AMutex<Box<dyn AtCommand + Send>>>,
    pub args: Vec<String>,
}

impl AtCommandCall {
    pub fn new(command: Arc<AMutex<Box<dyn AtCommand + Send>>>, args: Vec<String>) -> Self {
        AtCommandCall {
            command,
            args
        }
    }
}

pub async fn at_commands_dict() -> HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>> {
    return HashMap::from([
        ("@workspace".to_string(), Arc::new(AMutex::new(Box::new(AtWorkspace::new()) as Box<dyn AtCommand + Send>))),
        ("@file".to_string(), Arc::new(AMutex::new(Box::new(AtFile::new()) as Box<dyn AtCommand + Send>))),
        ("@ast_definition".to_string(), Arc::new(AMutex::new(Box::new(AtAstDefinition::new()) as Box<dyn AtCommand + Send>))),
        ("@ast_reference".to_string(), Arc::new(AMutex::new(Box::new(AtAstReference::new()) as Box<dyn AtCommand + Send>))),
        ("@lookup_symbols_at".to_string(), Arc::new(AMutex::new(Box::new(AtAstLookupSymbols::new()) as Box<dyn AtCommand + Send>))),
        // ("@ast_file_symbols".to_string(), Arc::new(AMutex::new(Box::new(AtAstFileSymbols::new()) as Box<dyn AtCommand + Send>))),
    ]);
}
