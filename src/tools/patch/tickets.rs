use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::ast::ast_structs::AstDefinition;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;
use crate::tools::patch::patch_utils::does_doc_have_symbol;


#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum PatchAction {
    #[default]
    AddToFile,
    RewriteSymbol,
    PartialEdit,
    RewriteWholeFile,
    NewFile,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum PatchLocateAs {
    BEFORE,
    AFTER,
    INFILL
}

impl PatchLocateAs {
    pub fn from_string(s: &str) -> Result<PatchLocateAs, String> {
        match s {
            "BEFORE" => Ok(PatchLocateAs::BEFORE),
            "AFTER" => Ok(PatchLocateAs::AFTER),
            "INFILL" => Ok(PatchLocateAs::INFILL),
            _ => Err(format!("invalid locate_as: {}", s)),
        }
    }
}

impl PatchAction {
    pub fn from_string(action: &str) -> Result<PatchAction, String> {
        match action {
            "📍ADD_TO_FILE" => Ok(PatchAction::AddToFile),
            "📍REWRITE_SYMBOL" => Ok(PatchAction::RewriteSymbol),
            "📍REWRITE_WHOLE_FILE" => Ok(PatchAction::RewriteWholeFile),
            "📍PARTIAL_EDIT" => Ok(PatchAction::PartialEdit),
            "📍NEW_FILE" => Ok(PatchAction::NewFile),
            "📍OTHER" => Ok(PatchAction::Other),
            _ => Err(format!("invalid action: {}", action)),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct TicketToApply {
    pub action: PatchAction, // action is changed for ADD_TO_FILE and REWRITE_SYMBOL if failed to parse
    pub orig_action: PatchAction,
    #[serde(default)]
    pub fallback_action: Option<PatchAction>,
    pub id: String,
    pub filename_before: String,
    pub filename_after: String,
    #[serde(default)]
    pub locate_as: Option<PatchLocateAs>,
    #[serde(default)]
    pub locate_symbol: Option<Arc<AstDefinition>>,
    #[serde(default)]
    pub all_symbols: Vec<Arc<AstDefinition>>,
    pub code: String,
}

pub async fn correct_and_validate_active_ticket(gcx: Arc<ARwLock<GlobalContext>>, ticket: &mut TicketToApply) -> Result<(), String> {
    fn good_error_text(reason: &str, ticket: &TicketToApply) -> String {
        format!("Failed to validate TICKET '{}': {}", ticket.id, reason)
    }
    async fn resolve_path(gcx: Arc<ARwLock<GlobalContext>>, path_str: &String) -> Result<String, String> {
        let candidates = file_repair_candidates(gcx.clone(), path_str, 10, false).await;
        return_one_candidate_or_a_good_error(gcx.clone(), path_str, &candidates, &get_project_dirs(gcx.clone()).await, false).await
    }

    let path_before = PathBuf::from(ticket.filename_before.as_str());
    let _path_after = PathBuf::from(ticket.filename_after.as_str());
    
    match ticket.action {
        PatchAction::AddToFile => {
            ticket.filename_before = resolve_path(gcx.clone(), &ticket.filename_before).await
                .map_err(|e| good_error_text(&format!("failed to resolve filename_before: '{}'. Error:\n{}", ticket.filename_before, e), ticket))?;
            ticket.fallback_action = Some(PatchAction::PartialEdit);
            if ticket.locate_as != Some(PatchLocateAs::BEFORE) && ticket.locate_as != Some(PatchLocateAs::AFTER) {
                ticket.action = PatchAction::PartialEdit;
            }
        },
        PatchAction::RewriteSymbol => {
            ticket.filename_before = resolve_path(gcx.clone(), &ticket.filename_before).await
               .map_err(|e| good_error_text(&format!("failed to resolve filename_before: '{}'. Error:\n{}", ticket.filename_before, e), ticket))?;
            ticket.fallback_action = Some(PatchAction::PartialEdit);

            if ticket.locate_as != Some(PatchLocateAs::INFILL) {
                ticket.action = PatchAction::PartialEdit;
            }
        },
        PatchAction::PartialEdit => {
            ticket.filename_before = resolve_path(gcx.clone(), &ticket.filename_before).await
                .map_err(|e| good_error_text(&format!("failed to resolve filename_before: '{}'. Error:\n{}", ticket.filename_before, e), ticket))?;
        },
        PatchAction::RewriteWholeFile => {
            ticket.filename_before = resolve_path(gcx.clone(), &ticket.filename_before).await
                .map_err(|e| good_error_text(&format!("failed to resolve filename_before: '{}'. Error:\n{}", ticket.filename_before, e), ticket))?;
        },
        PatchAction::NewFile => {
            if path_before.is_relative() {
                return Err(good_error_text(&format!("filename_before: '{}' must be absolute.", ticket.filename_before), ticket));
            }
        },
        PatchAction::Other => {}
    }
    Ok(())
}

async fn parse_tickets(gcx: Arc<ARwLock<GlobalContext>>, content: &str) -> Vec<TicketToApply> {
    async fn process_ticket(gcx: Arc<ARwLock<GlobalContext>>, lines: &[&str], line_num: usize) -> Result<(usize, TicketToApply), String> {
        let mut ticket = TicketToApply::default();
        let command_line = lines[line_num];
        let header = command_line.trim().split(" ").collect::<Vec<&str>>();
        
        ticket.action = match header.get(0) {
            Some(action) => {
                match PatchAction::from_string(action) {
                    Ok(a) => a,
                    Err(e) => return Err(format!("failed to parse ticket: couldn't parse TICKET ACTION.\nError: {e}"))
                }
            },
            None => return Err("failed to parse ticket, TICKET ACTION is missing".to_string()),
        };
        ticket.orig_action = ticket.action.clone();
        
        ticket.id = match header.get(1) {
            Some(id) => id.to_string(),
            None => return Err("failed to parse ticket, TICKED ID is missing".to_string()),
        };
        
        ticket.filename_before = match header.get(2) {
            Some(filename) => filename.to_string(),
            None => return Err("failed to parse ticket, TICKED FILENAME is missing".to_string()),
        };
        
        if let Some(el3) = header.get(3) {
            if let Ok(locate_as) = PatchLocateAs::from_string(el3) {
                ticket.locate_as = Some(locate_as);
            }
        }
        
        if let Some(el4) = header.get(4) {
            let locate_symbol_str = el4.to_string();
            match does_doc_have_symbol(gcx.clone(), &locate_symbol_str, &ticket.filename_before).await {
                Ok((symbol, all_symbols)) => {
                    ticket.locate_symbol = Some(symbol);
                    ticket.all_symbols = all_symbols;
                }
                Err(_) => {}
            }
        }

        if let Some(code_block_fence_line) = lines.get(line_num + 1) {
            if !code_block_fence_line.contains("```") {
                return Err("failed to parse ticket, invalid code block fence".to_string());
            }
            for (idx, line) in lines.iter().enumerate().skip(line_num + 2) {
                if line.contains("```") {
                    return Ok((2 + idx, ticket));
                }
                ticket.code.push_str(format!("{}\n", line).as_str());
            }
            Err("failed to parse ticket, no ending fence for the code block".to_string())
        } else {
            Err("failed to parse ticket, no code block".to_string())
        }
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut line_num = 0;
    let mut tickets = vec![];
    while line_num < lines.len() {
        let line = lines[line_num];
        if line.contains("📍") {
            match process_ticket(gcx.clone(), &lines, line_num).await {
                Ok((new_line_num, ticket)) => {
                    line_num = new_line_num;
                    tickets.push(ticket);
                }
                Err(err) => {
                    warn!("Skipping the ticket due to the error: {err}");
                    line_num += 1;
                    continue;
                }
            };
        } else {
            line_num += 1;
        }
    }
    
    tickets
}

pub async fn get_tickets_from_messages(
    ccx: Arc<AMutex<AtCommandsContext>>,
) -> HashMap<String, TicketToApply> {
    let (gcx, messages) = {
        let ccx_lock = ccx.lock().await;
        (ccx_lock.global_context.clone(), ccx_lock.messages.clone())
    };
    let mut tickets: HashMap<String, TicketToApply> = HashMap::new();
    for message in messages
        .iter()
        .filter(|x| x.role == "assistant") {
        for ticket in parse_tickets(gcx.clone(), &message.content).await.into_iter() {
            tickets.insert(ticket.id.clone(), ticket);
        }
    }
    tickets
}
