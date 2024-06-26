use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

use crate::at_tools::tools::AtTool;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::global_context::GlobalContext;

use crate::at_commands::at_workspace::AtWorkspace;
use crate::at_commands::at_file::AtFile;
use crate::at_commands::at_ast_definition::AtAstDefinition;
use crate::at_commands::at_ast_reference::AtAstReference;
use crate::at_commands::at_ast_lookup_symbols::AtAstLookupSymbols;
use crate::at_commands::at_file_search::AtFileSearch;
// use crate::at_commands::at_local_notes_to_self::AtLocalNotesToSelf;
use crate::at_commands::execute_at::AtCommandMember;


pub struct AtCommandsContext {
    pub global_context: Arc<ARwLock<GlobalContext>>,
    pub at_commands: HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>>,
    pub at_tools: HashMap<String, Arc<AMutex<Box<dyn AtTool + Send>>>>,
    pub top_n: usize,
    #[allow(dead_code)]
    pub is_preview: bool,
}

impl AtCommandsContext {
    pub async fn new(global_context: Arc<ARwLock<GlobalContext>>, top_n: usize, is_preview: bool) -> Self {
        AtCommandsContext {
            global_context: global_context.clone(),
            at_commands: at_commands_dict(global_context.clone()).await,
            at_tools: crate::at_tools::tools::at_tools_merged_and_filtered(global_context.clone()).await,
            top_n,
            is_preview
        }
    }
}

#[async_trait]
pub trait AtCommand: Send + Sync {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>;
    // returns (messages_for_postprocessing, text_on_clip)
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String>;
    fn depends_on(&self) -> Vec<String> { vec![] }   // "ast", "vecdb"
}

#[async_trait]
pub trait AtParam: Send + Sync {
    async fn is_value_valid(&self, value: &String, ccx: &AtCommandsContext) -> bool;
    async fn param_completion(&self, value: &String, ccx: &AtCommandsContext) -> Vec<String>;
    fn param_completion_valid(&self) -> bool {false}
}

pub async fn at_commands_dict(gcx: Arc<ARwLock<GlobalContext>>) -> HashMap<String, Arc<AMutex<Box<dyn AtCommand + Send>>>> {
    let at_commands_dict = HashMap::from([
        ("@workspace".to_string(), Arc::new(AMutex::new(Box::new(AtWorkspace::new()) as Box<dyn AtCommand + Send>))),
        ("@file".to_string(), Arc::new(AMutex::new(Box::new(AtFile::new()) as Box<dyn AtCommand + Send>))),
        ("@file-search".to_string(), Arc::new(AMutex::new(Box::new(AtFileSearch::new()) as Box<dyn AtCommand + Send>))),
        ("@definition".to_string(), Arc::new(AMutex::new(Box::new(AtAstDefinition::new()) as Box<dyn AtCommand + Send>))),
        ("@references".to_string(), Arc::new(AMutex::new(Box::new(AtAstReference::new()) as Box<dyn AtCommand + Send>))),
        ("@symbols-at".to_string(), Arc::new(AMutex::new(Box::new(AtAstLookupSymbols::new()) as Box<dyn AtCommand + Send>))),
        // ("@local-notes-to-self".to_string(), Arc::new(AMutex::new(Box::new(AtLocalNotesToSelf::new()) as Box<dyn AtCommand + Send>))),
    ]);

    let (ast_on, vecdb_on) = {
        let gcx = gcx.read().await;
        let vecdb = gcx.vec_db.lock().await;
        (gcx.ast_module.is_some(), vecdb.is_some())
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

    // Don't need custom at-commands?
    // let tconfig_maybe = crate::toolbox::toolbox_config::load_customization(gcx.clone()).await;
    // if tconfig_maybe.is_err() {
    //     error!("Error loading toolbox config: {:?}", tconfig_maybe.err().unwrap());
    // } else {
    //     for cust in tconfig_maybe.unwrap().tools {
    //         at_commands_dict.insert(
    //             format!("@{}", cust.name.clone()),
    //             Arc::new(AMutex::new(Box::new(AtExecuteCustCommand::new(
    //                 cust.command.clone(),
    //                 cust.timeout.clone(),
    //                 cust.postprocess.clone(),
    //             )) as Box<dyn AtCommand + Send>))
    //         );
    //     }
    // }
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

// pub fn filter_only_chat_messages_from_context_tool(tools: &Vec<ContextEnum>) -> Vec<ChatMessage> {
//     tools.iter()
//        .filter_map(|x| {
//             if let ContextEnum::ChatMessage(data) = x { Some(data.clone()) } else { None }
//         }).collect::<Vec<ChatMessage>>()
// }
