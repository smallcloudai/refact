use std::path::PathBuf;
use async_trait::async_trait;
use uuid::Uuid;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use std::sync::Arc;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::call_validation::ContextFile;


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

    async fn execute(&self, _query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ContextFile>, String), String> {
        let cache_dir = {
            let gcx_locked = context.global_context.read().await;
            gcx_locked.cache_dir.clone()
        };
        // join path cache_dir / notes
        let notes_dir_path = cache_dir.join("notes");
        // this all files
        let files = notes_dir_path.read_dir().map_err(|e| e.to_string())?;
        let mut files_vec: Vec<PathBuf> = vec![];
        let mut all_notes = String::new();
        for file_mb in files {
            if let Ok(file) = file_mb {
                files_vec.push(file.path());
            }
        }
        for file_path in files_vec {
            let file_text = std::fs::read_to_string(file_path.clone()).map_err(|e| e.to_string())?;
            let file_nameonly = file_path.file_name().unwrap().to_str().unwrap().to_string();
            all_notes.push_str(format!("{}\n```\n{}```", file_nameonly, file_text).as_str());
        }
        // 2024-05-22T17:03:51.366102Z  INFO refact_lsp::files_correction:99: not found Notes in cache_correction
        let context_file = ContextFile {
            file_name: "Notes".to_string(),
            file_content: all_notes.clone(),
            line1: 1,
            line2: all_notes.split("\n").collect::<Vec<&str>>().len(),
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness: 100.0,
            is_body_important: false
        };
        // let chat_message = ChatMessage {
        //     role: "assistant".to_string(),
        //     content: "Notes to self:".to_string() + "\n" + &all_notes,
        //     tool_calls: None,
        //     tool_call_id: "".to_string()
        // };
        Ok((vec![context_file], "".to_string()))
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


