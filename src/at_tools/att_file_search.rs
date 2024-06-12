use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file_search::{execute_at_file_search, text_on_clip};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttFileSearch;

#[async_trait]
impl AtTool for AttFileSearch {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let file_path = match args.get("file_path") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `file_path` is not a string: {:?}", v)),
            None => return Err("Missing argument `file_path` for att_file_search".to_string())
        };
        let query = match args.get("query") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("Missing argument `query` for att_file_search".to_string())
        };

        let vector_of_context_file = execute_at_file_search(ccx, &file_path, &query, true).await?;
        let text = text_on_clip(&query, &file_path, true);

        let mut results = vec_context_file_to_context_tools(vector_of_context_file);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));

        Ok(results)
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}