use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{info, warn};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::args_parser::parse_arguments;
use crate::at_tools::att_patch::chat_interaction::execute_chat_model;
use crate::at_tools::att_patch::diff_formats::parse_diff_chunks_from_message;
use crate::at_tools::att_patch::unified_diff_format::UnifiedDiffFormat;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};

pub const DEFAULT_MODEL_NAME: &str = "claude-3-5-sonnet";
pub const MAX_TOKENS: usize = 64000;
pub const MAX_NEW_TOKENS: usize = 8192;
pub const TEMPERATURE: f32 = 0.2;
pub type DefaultToolPatch = UnifiedDiffFormat;

pub struct ToolPatch {}
#[async_trait]
impl Tool for ToolPatch {
    async fn tool_execute(
        &self,
        ccx: &mut AtCommandsContext,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let args = match parse_arguments(args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Cannot parse input arguments: {err}. Try to call `patch` one more time with valid arguments"));
            }
        };
        let answer = match execute_chat_model(&args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Patch model execution problem: {err}. Try to call `patch` one more time"));
            }
        };
        info!("Tool patch answer:\n{answer}");
        match parse_diff_chunks_from_message(ccx, &answer).await {
            Ok(res) => {
                info!("Tool patch diff:\n{:?}", res);
                Ok(vec![(ContextEnum::ChatMessage(ChatMessage {
                    role: "diff".to_string(),
                    content: res,
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                }))])
            }
            Err(err) => {
                warn!(err);
                Err(format!("{err}. Try to call `patch` one more time to generate a correct diff"))
            }
        }
    }
}
