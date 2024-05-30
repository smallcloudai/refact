use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_execute_cmd::execute_cmd;
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttExecuteCommand {
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
}

#[async_trait]
impl AtTool for AttExecuteCommand {
    async fn execute(&self, _ccx: &mut AtCommandsContext, tool_call_id: &String, _args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        // TODO: use timeout as well
        let (stdout, stderr) = execute_cmd(&self.command).await?;
        
        let mut results = vec![ContextEnum::ChatMessage(ChatMessage::new(
            "assistant".to_string(),
            format!("{}{}", stdout, stderr),
        ))];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: format!("Attached log of the executed command: {}", self.command),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
}