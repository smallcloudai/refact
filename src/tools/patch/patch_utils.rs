use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use crate::ast::ast_db::doc_defs;
use crate::ast::ast_structs::AstDefinition;
use crate::global_context::GlobalContext;
use crate::tools::patch::tickets::TicketToApply;


pub fn vec_contains_vec<T: PartialEq>(vec: &[T], subvec: &[T]) -> usize {
    if subvec.is_empty() {
        return 0;
    }
    if subvec.len() > vec.len() {
        return 0;
    }
    vec.windows(subvec.len())
        .filter(|window| *window == subvec)
        .count()
}

pub fn most_common_value_in_vec<T: Eq + Hash + Copy>(items: Vec<T>) -> Option<T> {
    items.iter()
        .fold(HashMap::new(), |mut acc, &item| {
            *acc.entry(item).or_insert(0) += 1;
            acc
        })
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(item, _)| item)
}

pub fn minimal_common_indent(symbol_lines: &[&str]) -> (usize, usize) {
    let mut common_spaces = vec![];
    let mut common_tabs = vec![];
    for line in symbol_lines.iter().filter(|l|!l.is_empty()) {
        let spaces = line.chars().take_while(|c| *c == ' ').count();
        common_spaces.push(spaces);
        let tabs = line.chars().take_while(|c| *c == '\t').count();
        common_tabs.push(tabs);
    }
    (
        common_spaces.iter().min().cloned().unwrap_or(0),
        common_tabs.iter().min().cloned().unwrap_or(0)
    )
}

pub fn place_indent(code_lines: &[&str], indent_spaces: usize, indent_tabs: usize) -> Vec<String> {
    let (min_spaces, min_tabs) = minimal_common_indent(code_lines);

    code_lines.iter().map(|line| {
        let trimmed_line = line
            .chars()
            .skip(min_spaces + min_tabs)
            .collect::<String>();

        let new_indent = if line.is_empty() {"".to_string()} else {" ".repeat(indent_spaces) + &"\t".repeat(indent_tabs)};
        format!("{}{}", new_indent, trimmed_line)
    }).collect()
}

pub fn same_parent_symbols(ticket: &TicketToApply, locate_symbol: &Arc<AstDefinition>) -> Vec<Arc<AstDefinition>> {
    fn symbol_parent_elements(symbol: &Arc<AstDefinition>) -> Vec<String> {
        let mut elements = symbol.official_path.clone();
        elements.pop();
        elements
    }
    let mut grouped_symbols = HashMap::new();
    for symbol in &ticket.all_symbols {
        grouped_symbols.entry(symbol_parent_elements(symbol)).or_insert_with(Vec::new).push(symbol.clone());
    }
    let mut same_parents_syms = grouped_symbols.get(&symbol_parent_elements(locate_symbol)).cloned().unwrap_or(Vec::new());
    if same_parents_syms.len() > 1 {
        same_parents_syms.sort_by_key(|s| s.full_range.start_point.row);
    }
    same_parents_syms
}

pub fn most_common_spacing(same_parent_symbols: &Vec<Arc<AstDefinition>>) -> usize {
    return if same_parent_symbols.len() > 1 {
        let spacings: Vec<isize> = same_parent_symbols.windows(2)
            .map(|pair| {
                // info!("pair names: {:?} AND {:?}", pair[1].official_path, pair[0].official_path);
                // info!("diff: {}", pair[1].full_range.start_point.row as isize - pair[0].full_range.end_point.row as isize);
                (pair[1].full_range.start_point.row as isize - pair[0].full_range.end_point.row as isize).saturating_sub(1)
            })
            .collect();
        most_common_value_in_vec(spacings).unwrap_or(1) as usize
    } else {
        1
    }
}

pub async fn does_doc_have_symbol(
    gcx: Arc<ARwLock<GlobalContext>>,
    symbol: &String,
    doc_path: &String
) -> Result<(Arc<AstDefinition>, Vec<Arc<AstDefinition>>), String> {
    let symbol_parts = symbol.split("::").map(|s|s.to_string()).collect::<Vec<_>>();
    let ast_service = gcx.read().await.ast_service.clone()
        .ok_or("ast_service is absent".to_string())?;
    let ast_index = ast_service.lock().await.ast_index.clone();
    let doc_syms = doc_defs(ast_index, doc_path).await;
    let filtered_syms = doc_syms.iter().filter(|s|s.official_path.ends_with(&symbol_parts)).cloned().collect::<Vec<_>>();
    match filtered_syms.len() {
        0 => Err(format!("symbol '{}' not found in file '{}'", symbol, doc_path)),
        1 => Ok((filtered_syms[0].clone(), doc_syms)),
        _ => Err(format!("cannot locate symbol {}: multiple symbols found with this name" , symbol)),
    }
}
