use std::path::PathBuf;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use std::sync::Arc;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};


fn text_on_clip(from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    return "attached note to self".to_string();
}

pub struct AtLocalNotesToSelf {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtLocalNotesToSelf {
    pub fn new() -> Self {
        AtLocalNotesToSelf {
            params: vec![],
        }
    }
}

#[async_trait]
impl AtCommand for AtLocalNotesToSelf {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, _cmd: &mut AtCommandMember, _args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let cache_dir = {
            let gcx_locked = ccx.global_context.read().await;
            gcx_locked.cache_dir.clone()
        };
        let notes_dir_path = cache_dir.join("notes");
        let files = notes_dir_path.read_dir().map_err(|e| e.to_string())?;
        let mut files_vec: Vec<PathBuf> = vec![];
        for file_mb in files {
            if let Ok(file) = file_mb {
                files_vec.push(file.path());
            }
        }
        let mut context_tools = vec![];
        for file_path in files_vec {
            let file_text = std::fs::read_to_string(file_path.clone()).map_err(|e| e.to_string())?;
            let chat_message = ChatMessage::new(
                "assistant".to_string(),
                format!("Note to self: {}", file_text),
            );
            context_tools.push(ContextEnum::ChatMessage(chat_message));
        }
        let text = text_on_clip(false);
        Ok((context_tools, text))
    }
}
