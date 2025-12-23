use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::memories::{memories_add_enriched, EnrichmentParams};

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
            description: "Creates a new knowledge entry. Uses AI to enrich metadata and check for outdated documents.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "The knowledge content to store.".to_string(),
                },
                ToolParam {
                    name: "tags".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated tags (optional, will be auto-enriched).".to_string(),
                },
                ToolParam {
                    name: "filenames".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated related file paths (optional, will be auto-enriched).".to_string(),
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

        let content = match args.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `content` is not a string: {:?}", v)),
            None => return Err("argument `content` is missing".to_string()),
        };

        let user_tags: Vec<String> = match args.get("tags") {
            Some(Value::String(s)) => s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            _ => vec![],
        };

        let user_filenames: Vec<String> = match args.get("filenames") {
            Some(Value::String(s)) => s.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect(),
            _ => vec![],
        };

        let enrichment_params = EnrichmentParams {
            base_tags: user_tags,
            base_filenames: user_filenames,
            base_kind: "knowledge".to_string(),
            base_title: None,
        };

        let file_path = memories_add_enriched(ccx.clone(), &content, enrichment_params).await?;

        let result_msg = format!("Knowledge entry created: {}", file_path.display());

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(result_msg),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["knowledge".to_string()]
    }
}
