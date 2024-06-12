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
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) },
            None => { return Err("argument `path` is missing".to_string()) }
        };
        // TODO: optional line n
        // let line_n = match args.get("line") {
        //     Some(Value::Number(n)) if n.is_u64() => Some(n.as_u64().unwrap() as usize),
        //     Some(v) => return Err(format!("argument `line` is not a valid u64: {:?}", v)),
        //     None => return Err("line".to_string()),
        // };
        
        let mut results = vec![];
        let text = match execute_at_file(ccx, p.clone(), true).await {
            Ok(res) => {
                let text = text_on_clip(&res, true);
                results.extend(vec_context_file_to_context_tools(vec![res]));
                text
            },
            Err(e) => {
                e
            }
        };

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));

        Ok(results)
    }
}
