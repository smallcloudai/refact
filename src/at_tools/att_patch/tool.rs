use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;
use tracing::warn;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::args_parser::parse_arguments;
use crate::at_tools::att_patch::chat_interaction::execute_chat_model;
use crate::at_tools::att_patch::diff_formats::parse_diff_chunks_from_message;
use crate::at_tools::att_patch::unified_diff_format::UnifiedDiffFormat;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ChatUsage, ContextEnum};

pub const DEFAULT_MODEL_NAME: &str = "claude-3-5-sonnet";
pub const MAX_NEW_TOKENS: usize = 8192;
pub const TEMPERATURE: f32 = 0.2;
pub type DefaultToolPatch = UnifiedDiffFormat;


pub struct ToolPatch {
    pub usage: Option<ChatUsage>
}

impl ToolPatch {
    pub fn new() -> Self {
        ToolPatch {
            usage: None
        }
    }
}

#[async_trait]
impl Tool for ToolPatch {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let args = match parse_arguments(ccx.clone(), args).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Cannot parse input arguments: {err}. Try to call `patch` one more time with valid arguments"));
            }
        };
        let (answer, usage_mb) = match execute_chat_model(ccx.clone(), &args).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Patch model execution problem: {err}. Try to call `patch` one more time"));
            }
        };

        let mut results = vec![];

        let parsed_chunks = parse_diff_chunks_from_message(ccx.clone(), &answer).await.map_err(|err| {
            self.usage = usage_mb.clone();
            warn!(err);
            format!("{err}. Try to call `patch` one more time to generate a correct diff")
        })?;

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "diff".to_string(),
            content: parsed_chunks,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: usage_mb,
        }));

        Ok(results)
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
