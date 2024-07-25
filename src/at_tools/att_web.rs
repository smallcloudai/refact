use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext};
use crate::at_commands::at_web::{execute_at_web, text_on_clip};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};

pub struct AttWeb;

#[async_trait]
impl Tool for AttWeb {
    async fn tool_execute(&mut self, _ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let url = match args.get("url") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `url` is not a string: {:?}", v)),
            None => return Err("Missing argument `url` for att_web".to_string())
        };

        let text = execute_at_web(&url).await?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text_on_clip(&url),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        results.push(ContextEnum::ChatMessage(ChatMessage::new(
            "plain_text".to_string(),
            text,
        )));
        
        Ok(results)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
