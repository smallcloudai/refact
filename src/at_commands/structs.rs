use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::at_commands_dict;
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use crate::at_commands::at_file::AtFile;
use crate::at_commands::at_params::AtParamFilePath;
use crate::at_commands::at_workspace::AtWorkspace;
use crate::call_validation::ChatMessage;

pub struct AtCommandsContext {
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub at_commands: HashMap<String, Arc<AMutex<AtCommandKind>>>,
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
    fn params(&self) -> &Vec<Arc<AMutex<AtParamKind>>>;
    async fn are_args_valid(&self, args: &Vec<String>, context: &AtCommandsContext) -> Vec<bool>;
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool;
    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ChatMessage>, Value), String>;
}

#[async_trait]
pub trait AtParam {
    fn name(&self) -> &String;
    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool;
    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String>;
}

pub struct AtCommandCall {
    pub command: Arc<AMutex<AtCommandKind>>,
    pub args: Vec<String>,
}

impl AtCommandCall {
    pub fn new(command: Arc<AMutex<AtCommandKind>>, args: Vec<String>) -> Self {
        AtCommandCall {
            command,
            args
        }
    }
}

pub enum AtCommandKind {
    AtWorkspace(AtWorkspace),
    AtFile(AtFile),
}

#[async_trait]
impl AtCommand for AtCommandKind {
    fn name(&self) -> &String {
        match self {
            AtCommandKind::AtWorkspace(workspace) => workspace.name(),
            AtCommandKind::AtFile(file) => file.name(),
        }
    }

    fn params(&self) -> &Vec<Arc<AMutex<AtParamKind>>> {
        match self {
            AtCommandKind::AtWorkspace(workspace) => workspace.params(),
            AtCommandKind::AtFile(file) => file.params(),
        }
    }

    async fn are_args_valid(&self, args: &Vec<String>, context: &AtCommandsContext) -> Vec<bool> {
        match self {
            AtCommandKind::AtWorkspace(workspace) => workspace.are_args_valid(args, context).await,
            AtCommandKind::AtFile(file) => file.are_args_valid(args, context).await,
        }
    }
    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        match self {
            AtCommandKind::AtWorkspace(workspace) => workspace.can_execute(args, context).await,
            AtCommandKind::AtFile(file) => file.can_execute(args, context).await,
        }
    }

    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ChatMessage>, Value), String> {
        match self {
            AtCommandKind::AtWorkspace(workspace) => workspace.execute(query, args, top_n, context).await,
            AtCommandKind::AtFile(file) => file.execute(query, args, top_n, context).await,
        }
    }
}
pub enum AtParamKind {
    AtParamFilePath(AtParamFilePath),
}

#[async_trait]
impl AtParam for AtParamKind {
    fn name(&self) -> &String {
        match self {
            AtParamKind::AtParamFilePath(param) => param.name(),
        }
    }

    async fn is_value_valid(&self, value: &String, context: &AtCommandsContext) -> bool {
        match self {
            AtParamKind::AtParamFilePath(param) => param.is_value_valid(value, context).await,
        }
    }

    async fn complete(&self, value: &String, context: &AtCommandsContext, top_n: usize) -> Vec<String> {
        match self {
            AtParamKind::AtParamFilePath(param) => param.complete(value, context, top_n).await,
        }
    }
}
