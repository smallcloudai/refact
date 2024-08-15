use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::args_parser::parse_arguments;
use crate::at_tools::att_patch::chat_interaction::execute_chat_model;
use crate::at_tools::att_patch::diff_formats::parse_diff_chunks_from_message;
use crate::at_tools::att_patch::unified_diff_format::UnifiedDiffFormat;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ChatUsage, ContextEnum};

pub const DEFAULT_MODEL_NAME: &str = "gpt-4o-mini";
pub const MAX_NEW_TOKENS: usize = 8192;
pub const TEMPERATURE: f32 = 0.5;
pub const N_CHOICES: usize = 16;
pub type DefaultToolPatch = UnifiedDiffFormat;


pub struct ToolPatch {
    pub usage: Option<ChatUsage>,
}

impl ToolPatch {
    pub fn new() -> Self {
        ToolPatch {
            usage: None
        }
    }
}

fn choose_correct_chunk(chunks: Vec<Result<String, String>>) -> Result<String, String> {
    let errors = chunks
        .iter()
        .filter(|res| res.is_err())
        .map(|res| res.clone().unwrap_err())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        warn!("There is a list of errors for some generated diffs");
        for err in errors {
            warn!("{err}");
        }
    }
    if chunks.iter().all(|res| res.is_err()) {
        return Err("No valid chunks were generated".to_string());
    }

    let non_error_chunks = chunks
        .iter()
        .filter_map(|res| res.as_ref().ok())
        .cloned()
        .collect::<Vec<_>>();
    warn!("{} diff were parsed successfully", non_error_chunks.len());

    // return the most common chunk
    let mut chunks_freq = HashMap::new();
    for chunk in non_error_chunks.iter() {
        *chunks_freq.entry(chunk.as_str()).or_insert(0) += 1;
    }
    Ok(chunks_freq
        .iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, _)| k.to_string())
        .expect("see the logic above, this array should not be empty"))
}

#[async_trait]
impl Tool for ToolPatch {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<ContextEnum>, String> {
        let args = match parse_arguments(args).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Cannot parse input arguments: {err}. Try to call `patch` one more time with valid arguments"));
            }
        };
        let answers = match execute_chat_model(ccx.clone(), tool_call_id, &args).await {
            Ok(res) => res,
            Err(err) => {
                return Err(format!("Patch model execution problem: {err}. Try to call `patch` one more time"));
            }
        };

        let mut chunks_for_answers = vec![];
        for answer in answers.iter() {
            warn!("Patch model answer:\n{}", &answer);
            let parsed_chunks = parse_diff_chunks_from_message(ccx.clone(), &answer).await.map_err(|err| {
                warn!(err);
                format!("{err}. Try to call `patch` one more time to generate a correct diff")
            });
            chunks_for_answers.push(parsed_chunks);
        }
        let chunks = choose_correct_chunk(chunks_for_answers)?;

        Ok(vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "diff".to_string(),
                content: chunks,
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                usage: None  // TODO: add to subchat
            })
        ])
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
