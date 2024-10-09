use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;
use crate::tools::tool_patch_aux::diff_structs::chunks_from_diffs;
use crate::tools::tool_patch_aux::fs_utils::read_file;
use crate::tools::tool_patch_aux::postprocessing_utils::{minimal_common_indent, most_common_spacing, place_indent, same_parent_symbols};
use crate::tools::tool_patch_aux::tickets_parsing::{PatchLocateAs, TicketToApply};

pub async fn full_rewrite_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename_before.clone()).await
        .map_err(|e| format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
    let file_path = PathBuf::from(&context_file.file_name);

    let diffs = diff::lines(&context_file.file_content, &ticket.code);
    chunks_from_diffs(file_path, diffs)
}

pub async fn add_to_file_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename_before.clone()).await
        .map_err(|e| format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
    let context_file_path = PathBuf::from(&context_file.file_name);

    let symbol = ticket.locate_symbol.clone().ok_or("symbol is absent")?;

    let file_text = context_file.file_content.clone();
    let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
    let file_lines = file_text.split(line_ending).collect::<Vec<&str>>();
    let symbol_lines = file_lines[symbol.full_range.start_point.row..symbol.full_range.end_point.row].to_vec();
    let file_lines = file_lines.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let (indent_spaces, indent_tabs) = minimal_common_indent(&symbol_lines);

    let ticket_code = ticket.code.clone();
    let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
    let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
    let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);

    let locate_as = ticket.locate_as.clone().expect("locate_as not found");
    let same_parent_symbols = same_parent_symbols(ticket, &symbol);
    let pos_locate_symbol = same_parent_symbols.iter().position(|s| s.official_path == symbol.official_path).expect("symbol not found");
    let spacing = most_common_spacing(&same_parent_symbols);

    let new_code_lines = if locate_as == PatchLocateAs::BEFORE {
        let sym_before = if pos_locate_symbol == 0 { None } else { Some(same_parent_symbols[pos_locate_symbol - 1].clone()) };
        let sym_after = symbol;
        if let Some(sym_before) = sym_before {
            file_lines[..sym_before.full_range.end_point.row + 1].iter()
                .chain(vec!["".to_string(); spacing].iter())
                .chain(ticket_code_lines.iter())
                .chain(vec!["".to_string(); spacing].iter())
                .chain(file_lines[sym_after.full_range.start_point.row..].iter())
                .cloned().collect::<Vec<_>>()
        } else {
            file_lines[..sym_after.full_range.start_point.row].iter()
                .chain(ticket_code_lines.iter())
                .chain(vec!["".to_string(); spacing].iter())
                .chain(file_lines[sym_after.full_range.start_point.row..].iter())
                .cloned().collect::<Vec<_>>()
        }
    } else {
        let sym_before = symbol;
        let sym_after = same_parent_symbols.get(pos_locate_symbol + 1).cloned();
        if let Some(sym_after) = sym_after {
            file_lines[..sym_before.full_range.end_point.row + 1].iter()
                .chain(vec!["".to_string(); spacing].iter())
                .chain(ticket_code_lines.iter())
                .chain(vec!["".to_string(); spacing].iter())
                .chain(file_lines[sym_after.full_range.start_point.row..].iter())
                .cloned().collect::<Vec<_>>()
        } else {
            file_lines[..sym_before.full_range.end_point.row + 1].iter()
                .chain(vec!["".to_string(); spacing].iter())
                .chain(ticket_code_lines.iter())
                .chain(file_lines[sym_before.full_range.end_point.row + 1..].iter())
                .cloned().collect::<Vec<_>>()
        }
    };
    let new_code = new_code_lines.join(line_ending);

    let diffs = diff::lines(&context_file.file_content, &new_code);

    chunks_from_diffs(context_file_path, diffs)
}

pub async fn rewrite_symbol_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename_before.clone()).await
        .map_err(|e| format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
    let context_file_path = PathBuf::from(&context_file.file_name);
    let symbol = ticket.locate_symbol.clone().ok_or("symbol is absent")?;

    let file_text = context_file.file_content.clone();
    let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
    let file_lines = file_text.split(line_ending).collect::<Vec<&str>>();
    let symbol_lines = file_lines[symbol.full_range.start_point.row..symbol.full_range.end_point.row].to_vec();
    let (indent_spaces, indent_tabs) = minimal_common_indent(&symbol_lines);

    let ticket_code = ticket.code.clone();
    let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
    let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
    let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);

    let new_code_lines = file_lines[..symbol.full_range.start_point.row].iter()
        .map(|s| s.to_string())
        .chain(ticket_code_lines.iter().cloned())
        .chain(file_lines[symbol.full_range.end_point.row + 1..].iter().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    let new_code = new_code_lines.join(line_ending);

    let diffs = diff::lines(&context_file.file_content, &new_code);

    chunks_from_diffs(context_file_path, diffs)
}

pub fn new_file_diff(
    ticket: &TicketToApply,
) -> Vec<DiffChunk> {
    vec![
        DiffChunk {
            file_name: ticket.filename_before.clone(),
            file_name_rename: None,
            file_action: "add".to_string(),
            line1: 1,
            line2: 1,
            lines_remove: "".to_string(),
            lines_add: ticket.code.clone(),
            ..Default::default()
        }
    ]
}

