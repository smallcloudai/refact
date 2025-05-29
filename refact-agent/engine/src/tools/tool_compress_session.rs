use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc};

pub struct ToolCompressSession;

#[async_trait]
impl Tool for ToolCompressSession {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let file_paths = match args.get("file_paths") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `file_paths` is not a string: {:?}", v)),
            None => return Err("Missing argument `file_paths` in the compress_session() call.".to_string())
        };

        let files: Vec<String> = file_paths
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if files.is_empty() {
            return Err("No valid file paths provided in `file_paths`.".to_string());
        }

        let message = format!(
            "The following files will be compressed in the session:\n{}",
            files.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n")
        );

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
            name: "compress_session".to_string(),
            agentic: false,
            experimental: false,
            description: "Marks files for compression in the chat session. The chat compression logic will use this to compress the specified files.".to_string(),
            parameters: vec![
                crate::tools::tools_description::ToolParam {
                    name: "file_paths".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of file paths to compress in the session.".to_string(),
                }
            ],
            parameters_required: vec!["file_paths".to_string()],
        }
    }
}
