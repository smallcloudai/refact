use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_workspace::{execute_at_workspace, text_on_clip};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttWorkspace {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AttWorkspace {
    pub fn new() -> Self {
        AttWorkspace {
            params: vec![],
        }
    }
}

#[async_trait]
impl AtTool for AttWorkspace {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>
    {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let query = match args.get("query") {
            Some(query) => query.to_string(),
            None => return Err("Missing query argument for att_workspace".to_string())
        };
        let vector_of_context_file = execute_at_workspace(ccx, &query, &vec![]).await?;
        let text = text_on_clip(&query, true);

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
