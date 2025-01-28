use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, DiffChunk, SubchatParameters};
use crate::files_correction::to_pathbuf_normalize;
use crate::tools::tool_patch_aux::diff_apply::diff_apply;
use crate::tools::tool_patch_aux::model_based_edit::partial_edit::partial_edit_tickets_to_chunks;
use crate::tools::tool_patch_aux::no_model_edit::{full_rewrite_diff, rewrite_symbol_diff};
use crate::tools::tool_patch_aux::postprocessing_utils::postprocess_diff_chunks;
use crate::tools::tool_patch_aux::tickets_parsing::{get_and_correct_active_tickets, get_tickets_from_messages, good_error_text, PatchAction, TicketToApply};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use crate::tools::tools_execute::unwrap_subchat_params;
use crate::integrations::integr_abstract::IntegrationConfirmation;


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

pub async fn process_tickets(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    active_tickets: &mut Vec<TicketToApply>,
    ticket_ids: Vec<String>,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<DiffChunk>, (String, Option<String>)> {
    if active_tickets.is_empty() {
        return Ok(vec![]);
    }
    let gcx = ccx_subchat.lock().await.global_context.clone();
    let action = active_tickets[0].action.clone();
    let res = match action {
        PatchAction::RewriteSymbol => {
            match rewrite_symbol_diff(gcx.clone(), &active_tickets[0]).await {
                Ok(mut chunks) => {
                    postprocess_diff_chunks(gcx.clone(), &mut chunks)
                        .await
                        .map_err(|err| (err, None))
                }
                Err(err) => {
                    Err((err, None))
                }
            }
        }
        PatchAction::PartialEdit => {
            partial_edit_tickets_to_chunks(
                ccx_subchat.clone(), active_tickets.clone(), params, tool_call_id, usage,
            ).await
        }
        PatchAction::RewriteWholeFile => {
            match full_rewrite_diff(gcx.clone(), &active_tickets[0]).await {
                Ok(mut chunks) => {
                    postprocess_diff_chunks(gcx.clone(), &mut chunks)
                        .await.
                        map_err(|err| (err, None))
                }
                Err(err) => {
                    Err((err, None))
                }
            }
        }
        _ => {
            Err(good_error_text(&format!("unknown action provided: '{:?}'.", action), &ticket_ids, None))
        }
    };
    // todo: add multiple attempts for PartialEdit tickets (3)
    match res {
        Ok(_) => active_tickets.clear(),
        Err(_) => {
            // if AddToFile or RewriteSymbol failed => reassign them to PartialEdit
            active_tickets.retain(|x| x.fallback_action.is_some() && x.fallback_action != Some(x.action.clone()));
            active_tickets.iter_mut().for_each(|x| {
                if let Some(fallback_action) = x.fallback_action.clone() {
                    x.action = fallback_action;
                }
            });
        }
    }
    res
}

fn return_cd_instruction_or_error(
    err: &String,
    cd_instruction: &Option<String>,
    tool_call_id: &String,
    usage: &ChatUsage,
) -> Result<(bool, Vec<ContextEnum>), String> {
    if let Some(inst) = cd_instruction {
        tracing::info!("\n{}", inst);
        Ok((false, vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(err.to_string()),
                tool_calls: None,
                tool_call_id: tool_call_id.to_string(),
                usage: Some(usage.clone()),
                ..Default::default()
            }),
            ContextEnum::ChatMessage(ChatMessage {
                role: "cd_instruction".to_string(),
                content: ChatContent::SimpleText(inst.to_string()),
                tool_calls: None,
                tool_call_id: "".to_string(),
                usage: Some(usage.clone()),
                ..Default::default()
            })
        ]))
    } else {
        Err(err.to_string())
    }
}

fn parse_args(args: &HashMap<String, Value>) -> Result<(Vec<String>, String, Option<String>), String> {
    let tickets = match args.get("tickets") {
        Some(Value::String(s)) => s.split(",").map(|s| s.trim().to_string()).collect::<Vec<_>>(),
        Some(v) => { return Err(format!("argument 'ticket' should be a string: {:?}", v)) }
        None => { vec![] }
    };
    let path = match args.get("path") {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(v) => { return Err(format!("argument 'path' should be a string: {:?}", v)) }
        None => { return Err("argument 'path' is required".to_string()) }
    };
    let explanation = match args.get("explanation") {
        Some(Value::String(s)) => Some(s.trim().to_string()),
        Some(v) => { return Err(format!("argument 'explanation' should be a string: {:?}", v)) }
        None => None
    };
    if tickets.is_empty() {
        return Err("`tickets` shouldn't be empty".to_string());
    }
    Ok((tickets, path, explanation))
}

async fn create_ccx(ccx: Arc<AMutex<AtCommandsContext>>, params: &SubchatParameters) -> Result<Arc<AMutex<AtCommandsContext>>, String> {
    let ccx_lock = ccx.lock().await;
    Ok(Arc::new(AMutex::new(AtCommandsContext::new(
        ccx_lock.global_context.clone(),
        params.subchat_n_ctx,
        ccx_lock.top_n,
        false,
        ccx_lock.messages.clone(),
        ccx_lock.chat_id.clone(),
        ccx_lock.should_execute_remotely,
    ).await)))
}

async fn can_execute_patch(
    ccx: Arc<AMutex<AtCommandsContext>>,
    args: &HashMap<String, Value>,
) -> Result<(), String> {
    let (tickets, path, explanation_mb) = parse_args(args)?;
    let params = unwrap_subchat_params(ccx.clone(), "patch").await?;
    let ccx_subchat = create_ccx(ccx.clone(), &params).await?;

    let (gcx, messages) = {
        let ccx_lock = ccx_subchat.lock().await;
        (ccx_lock.global_context.clone(), ccx_lock.messages.clone())
    };

    let all_tickets_from_above = get_tickets_from_messages(gcx.clone(), &messages, explanation_mb).await;

    let active_tickets = get_and_correct_active_tickets(
        gcx.clone(),
        tickets.clone(),
        all_tickets_from_above.clone()
    ).await.map_err(|err| format!("Couldn't get active tickets from messages: {}", err.0))?;

    if active_tickets.is_empty() {
        return Err("no active tickets found".to_string());
    }

    if to_pathbuf_normalize(&active_tickets[0].filename_before) != to_pathbuf_normalize(&path) {
        return Err("the filename of the ticket(s) you provided doesn't match the filename of the file you provided".to_string());
    }

    Ok(())
}

#[async_trait]
impl Tool for ToolPatch {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (tickets, path, explanation_mb) = parse_args(args)?;
        let params = unwrap_subchat_params(ccx.clone(), "patch").await?;
        let ccx_subchat = create_ccx(ccx.clone(), &params).await?;

        let mut usage = ChatUsage { ..Default::default() };

        let (gcx, messages) = {
            let ccx_lock = ccx_subchat.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.messages.clone())
        };
        let all_tickets_from_above = get_tickets_from_messages(gcx.clone(), &messages, explanation_mb).await;
        let mut active_tickets = match get_and_correct_active_tickets(
            gcx.clone(),
            tickets.clone(),
            all_tickets_from_above.clone()
        ).await {
            Ok(res) => res,
            Err((err, cd_instruction)) => {
                return return_cd_instruction_or_error(&err, &cd_instruction, &tool_call_id, &usage);
            }
        };
        assert!(!active_tickets.is_empty());

        if to_pathbuf_normalize(&active_tickets[0].filename_before) != to_pathbuf_normalize(&path) {
            let (err, cd_instruction) = good_error_text(
                &format!("ticket(s) have different filename from what you provided: '{}'!='{}'.", active_tickets[0].filename_before, path),
                &tickets,
                Some("recreate the ticket with correct filename in 📍-notation or change path argument".to_string()),
            );
            return return_cd_instruction_or_error(&err, &cd_instruction, &tool_call_id, &usage);
        }
        let mut res;
        loop {
            let diff_chunks = process_tickets(
                ccx_subchat.clone(),
                &mut active_tickets,
                tickets.clone(),
                &params,
                tool_call_id,
                &mut usage,
            ).await;
            res = diff_chunks;
            if active_tickets.is_empty() {
                break;
            }
        }
        let mut diff_chunks = match res {
            Ok(res) => res,
            Err((err, cd_instruction)) => {
                return return_cd_instruction_or_error(&err, &cd_instruction, &tool_call_id, &usage);
            }
        };
        diff_apply(gcx.clone(), &mut diff_chunks).await.map_err(
            |err| format!("Couldn't apply the diff: {}", err)
        )?;
        let results = vec![
            ChatMessage {
                role: "diff".to_string(),
                content: ChatContent::SimpleText(json!(diff_chunks).to_string()),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                usage: Some(usage),
                ..Default::default()
            }
        ]
            .into_iter()
            .map(|x| ContextEnum::ChatMessage(x))
            .collect::<Vec<_>>();
        Ok((false, results))
    }

    async fn match_against_confirm_deny(&self, ccx: Arc<AMutex<AtCommandsContext>>, args: &HashMap<String, Value>) -> Result<MatchConfirmDeny, String> {
        let msgs_len = ccx.lock().await.messages.len();

        // workaround: if messages weren't passed by ToolsPermissionCheckPost, legacy
        if msgs_len != 0 {
            // if we cannot execute patch, there's no need for confirmation
            if let Err(_) = can_execute_patch(ccx.clone(), args).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "patch".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "patch".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("patch".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        return Some(IntegrationConfirmation {
            ask_user: vec!["patch*".to_string()],
            deny: vec![],
        });
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
