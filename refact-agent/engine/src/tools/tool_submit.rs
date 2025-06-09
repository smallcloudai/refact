use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc};

pub struct ToolSubmit;

#[async_trait]
impl Tool for ToolSubmit {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        _args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let message = "Task completed successfully. The interaction will now stop.".to_string();

        let mut results = Vec::new();
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "submit".to_string(),
            agentic: false,
            experimental: false,
            description: "Signals that the current task has been completed successfully. When called, this tool will stop the current interaction/conversation.".to_string(),
            parameters: vec![],
            parameters_required: vec![],
        }
    }
}
