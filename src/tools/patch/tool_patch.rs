use async_trait::async_trait;
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::patch::chat_interaction::execute_chat_model;
use crate::tools::patch::diff_formats::postprocess_diff_chunks_from_message;
use crate::tools::patch::snippets::{get_code_snippets, PatchAction};
use crate::tools::patch::unified_diff_format::UnifiedDiffFormat;
use crate::tools::patch::whole_file_diff::{full_rewrite_diff, new_file_diff};
use crate::tools::tools_execute::unwrap_subchat_params;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatUsage, ContextEnum, SubchatParameters};


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
        for err in errors.iter() {
            warn!("{err}");
        }
    }
    if chunks.iter().all(|res| res.is_err()) {
        let mut err_message = "No valid chunks were generated, reasons are:\n".to_string();
        for err in errors.iter().unique() {
            err_message.push_str(format!("- {err}\n").as_str());
        }
        err_message.push_str("Try to call `patch` one more time to generate a correct diff");
        return Err(err_message);
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
    let max_repeats = chunks_freq.iter().max_by_key(|(_, k)| *k).unwrap().1.clone();
    let chunks_max_repeats = chunks_freq
        .iter()
        .filter(|(_, v)| **v == max_repeats)
        .map(|x| *x.0)
        .collect::<Vec<_>>();
    Ok(chunks_max_repeats
        .iter()
        .max()
        .expect("There is no max repeats")
        .to_string())
}

async fn snippets2diff(
    ccx: Arc<AMutex<AtCommandsContext>>,
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    ticket: &String,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<String, String> {
    let snippets = get_code_snippets(ccx.clone()).await;
    let active_snippet = match snippets.get(ticket) {
        Some(s) => s,
        None => {
            return Err(format!("No code block found for the ticket {:?} did you forget to write one using 📍-notation?", ticket));
        }
    };
    match active_snippet.action {
        PatchAction::PartialEdit => {
            let mut all_chunks = match execute_chat_model(
                ccx_subchat.clone(),
                &active_snippet,
                &params.subchat_model,
                params.subchat_n_ctx,
                params.subchat_temperature,
                params.subchat_max_new_tokens,
                tool_call_id,
                usage,
            ).await {
                Ok(res) => res,
                Err(err) => {
                    return Err(format!("Patch model execution problem: {err}. Try to call `patch` one more time"));
                }
            };
            let mut chunks_for_answers = vec![];
            for chunks in all_chunks.iter_mut() {
                chunks_for_answers.push(postprocess_diff_chunks_from_message(ccx_subchat.clone(), chunks).await);
            }
            choose_correct_chunk(chunks_for_answers)
        }
        PatchAction::FullRewrite => {
            let mut chunks = full_rewrite_diff(ccx.clone(), &active_snippet).await?;
            postprocess_diff_chunks_from_message(ccx_subchat.clone(), &mut chunks).await
        }
        PatchAction::NewFile => {
            let mut chunks = new_file_diff(&active_snippet);
            postprocess_diff_chunks_from_message(ccx_subchat.clone(), &mut chunks).await
        }
        _ => {
            Err(format!(
                "cannot use `patch` with the given command `{:?}`",
                &active_snippet.action
            ))
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
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let ticket = match args.get("ticket") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `ticket` is not a string: {:?}", v)) }
            None => { "".to_string() }
        };

        let mut usage = ChatUsage { ..Default::default() };
        let params = unwrap_subchat_params(ccx.clone(), "patch").await?;
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            Arc::new(AMutex::new(AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                ccx_lock.top_n,
                false,
                ccx_lock.messages.clone(),
            ).await))
        };

        let diff = snippets2diff(
            ccx.clone(),
            ccx_subchat.clone(),
            &ticket,
            &params,
            tool_call_id,
            &mut usage,
        ).await?;

        Ok((false, vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "diff".to_string(),
                content: diff,
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                usage: Some(usage),
            })
        ]))
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
