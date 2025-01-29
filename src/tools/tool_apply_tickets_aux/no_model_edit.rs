use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;
use crate::tools::tool_apply_tickets_aux::diff_structs::chunks_from_diffs;
use crate::tools::tool_apply_tickets_aux::fs_utils::read_file;
use crate::tools::tool_apply_tickets_aux::postprocessing_utils::{minimal_common_indent, place_indent};
use crate::tools::tool_apply_tickets_aux::tickets_parsing::TicketToApply;

pub async fn full_rewrite_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    match read_file(gcx.clone(), ticket.filename.clone()).await {
        Ok(context_file) => {
            let file_path = PathBuf::from(&context_file.file_name);
            let diffs = diff::lines(&context_file.file_content, &ticket.code);
            chunks_from_diffs(file_path, diffs)
        }
        Err(_) => {
            Ok(vec![
                DiffChunk {
                    file_name: ticket.filename.clone(),
                    file_name_rename: None,
                    file_action: "add".to_string(),
                    line1: 1,
                    line2: 1,
                    lines_remove: "".to_string(),
                    lines_add: ticket.code.clone(),
                    ..Default::default()
                }
            ])
        }
    }
}

pub async fn rewrite_symbol_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename.clone()).await
        .map_err(|e| format!("cannot read file to modify: {}.\nError: {e}", ticket.filename))?;
    let context_file_path = PathBuf::from(&context_file.file_name);
    let symbol = ticket.locate_symbol.clone().ok_or("symbol is absent")?;

    let file_text = context_file.file_content.clone();
    let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
    let file_lines = file_text.split(line_ending).collect::<Vec<&str>>();
    let symbol_lines = file_lines[symbol.full_line1() - 1..symbol.full_line2()].to_vec();
    let (indent_spaces, indent_tabs) = minimal_common_indent(&symbol_lines);

    let ticket_code = ticket.code.clone();
    let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
    let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
    let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);

    let new_code_lines = file_lines[..symbol.full_line1() - 1].iter()
        .map(|s| s.to_string())
        .chain(ticket_code_lines.iter().cloned())
        .chain(file_lines[symbol.full_line2()..].iter().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    let new_code = new_code_lines.join(line_ending);

    let diffs = diff::lines(&context_file.file_content, &new_code);

    chunks_from_diffs(context_file_path, diffs)
}
