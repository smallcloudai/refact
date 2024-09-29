use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;
use crate::tools::patch::chat_interaction::read_file;
use crate::tools::patch::patch_utils::{minimal_common_indent, most_common_spacing, place_indent, same_parent_symbols, vec_contains_vec};
use crate::tools::patch::tickets::{PatchAction, PatchLocateAs, TicketToApply};
use crate::tools::patch::unified_diff_format::{diff_blocks_to_diff_chunks, DiffBlock, DiffLine, LineType};


pub async fn full_rewrite_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename_before.clone()).await
        .map_err(|e|format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
    let file_path = PathBuf::from(&context_file.file_name);

    let diffs = diff::lines(&context_file.file_content, &ticket.code);
    chunks_from_diffs(file_path, diffs)
}

pub async fn add_to_file_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    ticket: &TicketToApply,
) -> Result<Vec<DiffChunk>, String> {
    let context_file = read_file(gcx.clone(), ticket.filename_before.clone()).await
        .map_err(|e|format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
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
        .map_err(|e|format!("cannot read file to modify: {}.\nError: {e}", ticket.filename_before))?;
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

fn chunks_from_diffs(file_path: PathBuf, diffs: Vec<diff::Result<&str>>) -> Result<Vec<DiffChunk>, String> {
    let mut line_num: usize = 0;
    let mut blocks = vec![];
    let mut diff_lines = vec![];
    for diff in diffs {
        match diff {
            diff::Result::Left(l) => {
                diff_lines.push(DiffLine {
                    line: l.to_string(),
                    line_type: LineType::Minus,
                    file_line_num_idx: Some(line_num),
                    correct_spaces_offset: Some(0),
                });
                line_num += 1;
            }
            diff::Result::Right(r) => {
                diff_lines.push(DiffLine {
                    line: r.to_string(),
                    line_type: LineType::Plus,
                    file_line_num_idx: Some(line_num),
                    correct_spaces_offset: Some(0),
                });
            }
            diff::Result::Both(_, _) => {
                line_num += 1;
                if !diff_lines.is_empty() {
                    blocks.push(DiffBlock {
                        file_name_before: file_path.clone(),
                        file_name_after: file_path.clone(),
                        action: "edit".to_string(),
                        file_lines: Arc::new(vec![]),
                        hunk_idx: 0,
                        diff_lines: diff_lines.clone(),
                    });
                    diff_lines.clear();
                }
            }
        }
    }
    if !diff_lines.is_empty() {
        blocks.push(DiffBlock {
            file_name_before: file_path.clone(),
            file_name_after: file_path.clone(),
            action: "edit".to_string(),
            file_lines: Arc::new(vec![]),
            hunk_idx: 0,
            diff_lines: diff_lines.clone(),
        });
        diff_lines.clear();
    }

    Ok(diff_blocks_to_diff_chunks(&blocks))
}

pub async fn retain_non_applied_tickets(
    gcx: Arc<ARwLock<GlobalContext>>,
    active_tickets: &mut Vec<TicketToApply>,
) {
    let action = active_tickets[0].action.clone();
    match action {
        PatchAction::AddToFile => {
            let ticket = &active_tickets[0];
            if let Ok(context_file) = read_file(gcx.clone(), ticket.filename_before.clone()).await {
                let symbol = match ticket.locate_symbol.clone() {
                    Some(s) => s,
                    None => { return }
                };
                let file_text = context_file.file_content.clone();
                let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
                let file_lines = file_text.split(line_ending).collect::<Vec<&str>>();
                let symbol_lines = file_lines[symbol.full_range.start_point.row..symbol.full_range.end_point.row].to_vec();
                let (indent_spaces, indent_tabs) = minimal_common_indent(&symbol_lines);

                let ticket_code = ticket.code.clone();
                let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
                let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
                let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);

                let mut all_symbols = ticket.all_symbols.clone();
                all_symbols.sort_by_key(|s| s.full_range.start_point.row);
                let locate_as = ticket.locate_as.clone().expect("locate_as not found");

                let search_in_code = match locate_as {
                    PatchLocateAs::BEFORE => {
                        Some(file_lines[..symbol.full_range.start_point.row].to_vec())
                    },
                    PatchLocateAs::AFTER => {
                        Some(file_lines[symbol.full_range.end_point.row..].to_vec())
                    },
                    _ => None
                };
                if let Some(search_in_code) = search_in_code {
                    if vec_contains_vec(
                        &search_in_code.into_iter().map(|x|x.to_string()).collect::<Vec<_>>(),
                        &ticket_code_lines
                    ) == 1 {
                        active_tickets.clear();
                    }
                }
            }
        },
        PatchAction::RewriteSymbol => {
            let ticket = &active_tickets[0];
            if let Ok(context_file) = read_file(gcx.clone(), ticket.filename_before.clone()).await {
                let file_text = context_file.file_content.clone();
                let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
                let file_lines = file_text.split(line_ending).collect::<Vec<&str>>();

                match ticket.locate_symbol.clone() {
                    Some(symbol) => {
                        let symbol_lines = file_lines[symbol.full_range.start_point.row..symbol.full_range.end_point.row].to_vec();
                        let (indent_spaces, indent_tabs) = minimal_common_indent(&symbol_lines);

                        let ticket_code = ticket.code.clone();
                        let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
                        let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
                        let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);
                        
                        if vec_contains_vec(
                            &ticket_code_lines,
                            &symbol_lines.iter().map(|x|x.to_string()).collect::<Vec<_>>()
                        ) == 1 {
                            active_tickets.clear();
                        }
                    },
                    None => {
                        let (indent_spaces, indent_tabs) = minimal_common_indent(&file_lines);

                        let ticket_code = ticket.code.clone();
                        let ticket_line_ending = if ticket_code.contains("\r\n") { "\r\n" } else { "\n" };
                        let ticket_code_lines = ticket_code.split(ticket_line_ending).collect::<Vec<&str>>();
                        let ticket_code_lines = place_indent(&ticket_code_lines, indent_spaces, indent_tabs);
                        
                        if vec_contains_vec(
                            &file_lines.into_iter().map(|x|x.to_string()).collect::<Vec<_>>(),
                            &ticket_code_lines
                        ) == 1 {
                            active_tickets.clear();
                        }
                    }
                };
            }
        },
        PatchAction::PartialEdit => {
            // todo: implement
        },
        PatchAction::RewriteWholeFile => {
            let ticket = &active_tickets[0];
            if let Ok(context_file) = read_file(gcx.clone(), ticket.filename_before.clone()).await {
                let line_ending = if context_file.file_content.contains("\r\n") { "\r\n" } else { "\n" };
                let mut file_content = context_file.file_content.clone();
                file_content.push_str(line_ending);
                if ticket.code == file_content {
                    active_tickets.clear();
                }
            }
        },
        PatchAction::NewFile => {
            let ticket = &active_tickets[0];
            let path = PathBuf::from(&ticket.filename_before.clone());
            if path.is_file() {
                active_tickets.clear();
            }
        },
        _ => {}
    }
}
