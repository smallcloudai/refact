use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};

pub struct ToolCreateKnowledge {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolCreateKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "create_knowledge".to_string(),
            display_name: "Create Knowledge".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Creates a new knowledge entry as a markdown file in the project's .refact_knowledge folder.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "tags".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated tags for categorizing the knowledge entry, e.g. \"architecture, patterns, rust\"".to_string(),
                },
                ToolParam {
                    name: "filenames".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of related file paths that this knowledge entry documents or references.".to_string(),
                },
                ToolParam {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "The knowledge content to store. Include comprehensive information about implementation details, code patterns, architectural decisions, or solutions.".to_string(),
                },
            ],
            parameters_required: vec!["content".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("create_knowledge {:?}", args);

        let gcx = ccx.lock().await.global_context.clone();

        let content = match args.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `content` is not a string: {:?}", v)),
            None => return Err("argument `content` is missing".to_string()),
        };

        let tags: Vec<String> = match args.get("tags") {
            Some(Value::String(s)) => s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
            Some(Value::Array(arr)) => arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => vec!["knowledge".to_string()],
        };
        let tags = if tags.is_empty() { vec!["knowledge".to_string()] } else { tags };

        let filenames: Vec<String> = match args.get("filenames") {
            Some(Value::String(s)) => s.split(',')
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect(),
            _ => vec![],
        };

        let file_path = crate::memories::memories_add(gcx, &tags, &filenames, &content).await?;

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(format!("Knowledge entry created: {}", file_path.display())),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
