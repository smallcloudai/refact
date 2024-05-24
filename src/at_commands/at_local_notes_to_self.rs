use std::path::PathBuf;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use std::sync::Arc;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::call_validation::{ChatMessage, ContextTool};

fn text_on_clip(from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    return "attached note to self".to_string();
}

pub struct AtLocalNotesToSelf {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtLocalNotesToSelf {
    pub fn new() -> Self {
        AtLocalNotesToSelf {
            name: "@local-notes-to-self".to_string(),
            params: vec![],
        }
    }
}

#[async_trait]
impl AtCommand for AtLocalNotesToSelf {
    fn name(&self) -> &String {
        &self.name
    }

    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn execute(&self, _query: &String, _args: &Vec<String>, _top_n: usize, context: &AtCommandsContext, from_tool_call: bool) -> Result<(Vec<ContextTool>, String), String> {
        let cache_dir = {
            let gcx_locked = context.global_context.read().await;
            gcx_locked.cache_dir.clone()
        };
        // join path cache_dir / notes
        let notes_dir_path = cache_dir.join("notes");
        // this all files
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
            context_tools.push(ContextTool::ChatMessage(chat_message));
        }
        let text = text_on_clip(from_tool_call);
        Ok((context_tools, text))
    }
}


// [
// {
//     role: "assistant",
//     content: "Let's look up Frog and Toad",
//     tool_calls: [{"Frog", "defintion", id="xxx"}, {"Toad", "references", id="yyyy"}]
// },
// {
//     role: "tool",
//     content: "`Frog` defined in frog.py and other files",
//     call_id: "xxx",
// },
// {
//     role: "tool",
//     content: "`Toad` referenced in 1.py:1337 2.py:1338 and 25 other places",
//     call_id: "yyyy",
// },
// {
//     role: "context_file",
//     content: "[{frog.py\n```xxxxxxx```}, {toad.py\n```yyyyyyyy```}]",
//     call_id
// },
// ]


