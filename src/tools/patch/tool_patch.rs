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
use crate::tools::patch::snippets::{CodeSnippet, correct_and_validate_code_snippet, get_code_snippets, PatchAction};
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

async fn partial_edit_snippets_to_diffs(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    snippets: Vec<CodeSnippet>,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<String, String>{
    let mut all_chunks = execute_chat_model(
        ccx_subchat.clone(),
        snippets,
        &params.subchat_model,
        params.subchat_n_ctx,
        params.subchat_temperature,
        params.subchat_max_new_tokens,
        tool_call_id,
        usage,
    ).await.map_err(|e|format!("Failed to execute patch. Reason: failed communicate with the chat model.\nERROR: {e}"))?;
    
    let mut chunks_for_answers = vec![];
    for chunks in all_chunks.iter_mut() {
        let diffs = postprocess_diff_chunks_from_message(ccx_subchat.clone(), chunks).await;
        chunks_for_answers.push(diffs);
    }
    choose_correct_chunk(chunks_for_answers)
}

async fn snippets2diff(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    snippets: HashMap<String, CodeSnippet>,
    tickets: Vec<String>,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<String, String> {
    let gcx = ccx_subchat.lock().await.global_context.clone();

    fn good_error_text(reason: &str, snippets: Option<&Vec<CodeSnippet>>) -> String {
        let mut text = format!("Failed to execute patch. Reason: {reason}");
        if let Some (snippets) = snippets{
            let snippets_printed = snippets.iter().map(|s|format!("TICKET {}; ACTION: {:?}", s.ticket, s.action)).collect::<Vec<_>>().join("\n");
            text.push_str(&format!("\nTickets provided:\n{}", snippets_printed));
        }
        text
    }
    let mut active_snippets = tickets.iter().map(|t|snippets.get(t).cloned()
        .ok_or(good_error_text("No code block found for the ticket {:?} did you forget to write one using 📍-notation?", None))
    ).collect::<Result<Vec<_>, _>>()?;
    drop(snippets);
    for snippet in active_snippets.iter_mut() {
        correct_and_validate_code_snippet(gcx.clone(), snippet).await.map_err(|e|good_error_text(&e, None))?;
    }

    if active_snippets.is_empty() {
        return Err(good_error_text("no snippets that are referred by tickets were found.", Some(&active_snippets)));
    }
    if active_snippets.len() > 1 && !active_snippets.iter().all(|s|PatchAction::PartialEdit == s.action) {
        return Err(good_error_text("multiple tickets is allowed only for action==PARTIAL_EDIT.", Some(&active_snippets)));
    }
    if active_snippets.iter().map(|s|s.action.clone()).unique().count() > 1 {
        return Err(good_error_text("tickets must have the same action.", Some(&active_snippets)));
    }
    
    let action = active_snippets[0].action.clone();
    let result = match action {
        PatchAction::PartialEdit => partial_edit_snippets_to_diffs(
            ccx_subchat.clone(), active_snippets, params, tool_call_id, usage
        ).await,
        PatchAction::FullRewrite => {
            let mut chunks = full_rewrite_diff(ccx_subchat.clone(), &active_snippets[0]).await?;
            postprocess_diff_chunks_from_message(ccx_subchat.clone(), &mut chunks).await
        },
        PatchAction::NewFile => {
            let mut chunks = new_file_diff(&active_snippets[0]);
            postprocess_diff_chunks_from_message(ccx_subchat.clone(), &mut chunks).await
        },
        _ => Err(good_error_text(&format!("unknown action provided: '{:?}'.", action), Some(&active_snippets)))
    }?;
    
    Ok(result)
}

#[async_trait]
impl Tool for ToolPatch {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let tickets = match args.get("tickets") {
            Some(Value::String(s)) => s.split(",").map(|s|s.trim().to_string()).collect::<Vec<_>>(),
            Some(v) => { return Err(format!("argument 'ticket' should be a string: {:?}", v)) }
            None => { vec![] }
        };
        if tickets.is_empty() {
            return Err("`tickets` shouldn't be empty".to_string());
        }

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

        let snippets = get_code_snippets(ccx.clone()).await;
        let diff = snippets2diff(
            ccx_subchat,
            snippets,
            tickets,
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
