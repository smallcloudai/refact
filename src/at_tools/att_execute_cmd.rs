use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_local_cmdline::execute_cmd;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttExecuteCommand {
    pub command: String,
    pub timeout: usize,
    pub postprocess: String,
}

#[async_trait]
impl Tool for AttExecuteCommand {
    async fn execute(&self, _ccx: &mut AtCommandsContext, tool_call_id: &String, _args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let (stdout, stderr) = execute_cmd(&self.command, self.timeout).await?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: format!("Running compile:\n```{}{}```", stdout, stderr),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
}
