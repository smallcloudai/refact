use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AtNoteToSelf {
}

#[async_trait]
impl AtTool for AtNoteToSelf {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String>
    {
        let cache_dir = {
            let gcx_locked = ccx.global_context.read().await;
            gcx_locked.cache_dir.clone()
        };
        let notes_dir_path = cache_dir.join("notes");

        let text = match args.get("text") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `text` is not a string: {:?}", v)) },
            None => { return Err("argument `text` is not a string".to_string()) }
        };

        // open file "note20240531.txt" and write arg0 to it
        let fname = notes_dir_path.join(format!(
            "note{}_{}.txt",
            chrono::Local::now().format("%Y%m%d"),
            tool_call_id
        ));
        let file_maybe = tokio::fs::File::create(fname.clone()).await;
        if file_maybe.is_err() {
            return Err(format!("Error creating file {}", fname.clone().display()));
        }

        let mut buf = String::new();
        buf.push_str(text.as_str());
        buf.push_str("\n");
        let did_it_work = file_maybe.unwrap().write_all(buf.as_bytes()).await;
        if did_it_work.is_err() {
            return Err(format!("Error writing to file {}", fname.clone().display()));
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: format!("Note saved"),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
}
