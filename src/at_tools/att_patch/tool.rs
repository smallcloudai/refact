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

pub const DEFAULT_MODEL_NAME: &str = "gpt-4o";
pub const MAX_TOKENS: usize = 32000;
pub const MAX_NEW_TOKENS: usize = 8192;
pub const TEMPERATURE: f32 = 0.0;
pub type DefaultToolPatch = UnifiedDiffFormat;

pub struct ToolPatch {}
#[async_trait]
impl Tool for ToolPatch {
    async fn execute(
        &self,
        ccx: &mut AtCommandsContext,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let args = match parse_arguments(args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        let answer = match execute_chat_model(&args, ccx).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        info!("Tool patch answer: {answer}");
        match parse_diff_chunks_from_message(ccx, &answer).await {
            Ok(res) => {
                info!("Tool patch diff: {:?}", res);
                Ok(vec![(ContextEnum::ChatMessage(ChatMessage {
                    role: "diff".to_string(),
                    content: res,
                    tool_calls: None,
                    tool_call_id: tool_call_id.clone(),
                }))])
            }
            Err(err) => {
                warn!(err);
                Err(format!("Can't make any changes: {err}"))
            }
        }
    }
}
