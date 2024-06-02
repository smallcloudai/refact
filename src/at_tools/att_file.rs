use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{execute_at_file, text_on_clip};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttFile;

#[async_trait]
impl AtTool for AttFile {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let p = match args.get("path") {
            Some(x) => x.to_string().clone(),
            None => { return Err("missing file path".to_string()); }
        };

        let context_file = execute_at_file(ccx, p).await?;
        let text = text_on_clip(&context_file, true);

        let mut results = vec_context_file_to_context_tools(vec![context_file]);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));

        Ok(results)
    }
}
