use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_web::execute_at_web;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};


pub struct ToolWeb {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolWeb {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "web".to_string(),
            display_name: "Web".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Fetch a web page and convert to readable plain text.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "url".to_string(),
                    description: "URL of the web page to fetch.".to_string(),
                    param_type: "string".to_string(),
                },
            ],
            parameters_required: vec!["url".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let url = match args.get("url") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `url` is not a string: {:?}", v)),
            None => return Err("Missing argument `url` for att_web".to_string())
        };

        let text = execute_at_web(&url).await?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(text),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
