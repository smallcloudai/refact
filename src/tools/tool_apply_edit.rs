use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, DiffChunk, SubchatParameters};
use crate::tools::tool_apply_edit_aux::diff_apply::diff_apply;
use crate::tools::tool_apply_edit_aux::no_model_edit::{full_rewrite_diff, rewrite_symbol_diff};
use crate::tools::tool_apply_edit_aux::postprocessing_utils::postprocess_diff_chunks;
use crate::tools::tool_apply_edit_aux::tickets_parsing::{validate_and_correct_ticket, get_tickets_from_messages, good_error_text, PatchAction, TicketToApply};
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool};
use crate::tools::tools_execute::unwrap_subchat_params;
use crate::integrations::integr_abstract::IntegrationConfirmation;
use crate::tools::tool_apply_edit_aux::model_based_edit::model_execution::section_edit_tickets_to_chunks;

pub struct ToolApplyEdit {
    pub usage: Option<ChatUsage>,
}

impl ToolApplyEdit {
    pub fn new() -> Self {
        ToolApplyEdit {
            usage: None
        }
    }
}

pub async fn process_ticket(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    ticket: &mut TicketToApply,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<DiffChunk>, (String, Option<String>)> {
    let gcx = ccx_subchat.lock().await.global_context.clone();
    loop {
        let res = match &ticket.action {
            PatchAction::ReplaceSymbol => {
                match rewrite_symbol_diff(gcx.clone(), &ticket).await {
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
            PatchAction::SectionEdit => {
                section_edit_tickets_to_chunks(ccx_subchat.clone(), ticket, params, tool_call_id, usage).await
            }
            PatchAction::ReplaceFile => {
                match full_rewrite_diff(gcx.clone(), &ticket).await {
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
            PatchAction::DeleteFile => {
                Ok(vec![
                    DiffChunk {
                        file_name: ticket.filename.clone(),
                        file_name_rename: None,
                        file_action: "remove".to_string(),
                        line1: 1,
                        line2: 1,
                        lines_remove: ticket.code.clone(),
                        lines_add: "".to_string(),
                        ..Default::default()
                    }
                ])
            }
            _ => {
                Err(good_error_text(&format!("unknown action provided: '{:?}'.", ticket.action), &ticket.id, None))
            }
        };
        match res {
            Ok(_) => return res,
            Err(err) => {
                // if ReplaceSymbol failed => reassign them to SectionEdit
                if let Some(fallback_action) = ticket.fallback_action.clone() {
                    ticket.action = fallback_action;
                    ticket.fallback_action = None;
                    continue;
                } else {
                    return Err(err)
                }
            }
        }
    }
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

fn parse_args(args: &HashMap<String, Value>) -> Result<(String, Option<String>), String> {
    let ticket = match args.get("ticket") {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(v) => { return Err(format!("argument 'ticket' should be a string: {:?}", v)) }
        None => { return Err("argument 'ticket' is required".to_string()) }
    };
    let location_hints = match args.get("location_hints") {
        Some(Value::String(s)) => Some(s.trim().to_string()),
        Some(v) => { return Err(format!("argument 'location_hints' should be a string: {:?}", v)) }
        None => None
    };
    Ok((ticket, location_hints))
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
    let (ticket_id, location_hints_mb) = parse_args(args)?;
    let params = unwrap_subchat_params(ccx.clone(), "apply_edit").await?;
    let ccx_subchat = create_ccx(ccx.clone(), &params).await?;
    let (gcx, messages) = {
        let ccx_lock = ccx_subchat.lock().await;
        (ccx_lock.global_context.clone(), ccx_lock.messages.clone())
    };
    let all_tickets = get_tickets_from_messages(gcx.clone(), &messages, location_hints_mb).await;
    let _ = validate_and_correct_ticket(gcx.clone(), ticket_id, &all_tickets).await.map_err(|(err, _)| err)?;
    Ok(())
}

#[async_trait]
impl Tool for ToolApplyEdit {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (ticket_id, location_hints_mb) = parse_args(args)?;
        let params = unwrap_subchat_params(ccx.clone(), "apply_edit").await?;
        let ccx_subchat = create_ccx(ccx.clone(), &params).await?;

        let mut usage = ChatUsage { ..Default::default() };

        let (gcx, messages) = {
            let ccx_lock = ccx_subchat.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.messages.clone())
        };
        let all_tickets_from_above = get_tickets_from_messages(gcx.clone(), &messages, location_hints_mb).await;
        let mut ticket = match validate_and_correct_ticket(gcx.clone(), ticket_id, &all_tickets_from_above).await {
            Ok(res) => res,
            Err((err, cd_instruction)) => {
                return return_cd_instruction_or_error(&err, &cd_instruction, &tool_call_id, &usage);
            }
        };
        let mut diff_chunks = match process_ticket(
            ccx_subchat.clone(),
            &mut ticket,
            &params,
            tool_call_id,
            &mut usage,
        ).await {
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
            // if we cannot execute apply_edit, there's no need for confirmation
            if let Err(_) = can_execute_patch(ccx.clone(), args).await {
                return Ok(MatchConfirmDeny {
                    result: MatchConfirmDenyResult::PASS,
                    command: "apply_edit".to_string(),
                    rule: "".to_string(),
                });
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: "apply_edit".to_string(),
            rule: "default".to_string(),
        })
    }

    fn command_to_match_against_confirm_deny(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("apply_edit".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["apply_edit*".to_string()],
            deny: vec![],
        })
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        &mut self.usage
    }
}
