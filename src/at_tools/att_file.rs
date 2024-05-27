use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{AtParamFilePath, execute_at_file, text_on_clip};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AttFile {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AttFile {
    pub fn new() -> Self {
        AttFile {
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

#[async_trait]
impl AtTool for AttFile {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let file_path = match args.get("file_path") {
            Some(x) => x.to_string().clone(),
            None => { return Err("missing file path".to_string()); }
        };
        
        let context_file = execute_at_file(ccx, file_path).await?;
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
