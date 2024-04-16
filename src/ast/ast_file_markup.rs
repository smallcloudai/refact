use std::sync::Arc;
use std::collections::HashMap;
use std::cell::RefCell;
use uuid::Uuid;
use crate::ast::structs::FileASTMarkup;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, read_symbol, SymbolInformation};
use crate::files_in_workspace::Document;


pub async fn lowlevel_file_markup(
    doc: &Document,
    symbols: &Vec<AstSymbolInstanceArc>,
) -> Result<FileASTMarkup, String> {
    assert!(doc.text.is_some());
    let mut symbols4export: Vec<Arc<RefCell<SymbolInformation>>> = symbols.iter().map(|s| {
        let s_ref = read_symbol(s);
        // filter is_declaration?
        Arc::new(RefCell::new(s_ref.symbol_info_struct()))
    }).collect();
    let guid_to_symbol: HashMap<Uuid, Arc<RefCell<SymbolInformation>>> = symbols4export.iter().map(
        |s| (s.borrow().guid.clone(), s.clone())
    ).collect();
    fn recursive_path_of_guid(guid_to_symbol: &HashMap<Uuid, Arc<RefCell<SymbolInformation>>>, guid: &Uuid) -> String
    {
        return match guid_to_symbol.get(guid) {
            Some(x) => {
                let pname = if !x.borrow().name.is_empty() { x.borrow().name.clone() } else { x.borrow().guid.to_string()[..8].to_string() };
                let pp = recursive_path_of_guid(&guid_to_symbol, &x.borrow().parent_guid);
                format!("{}::{}", pp, pname)
            }
            None => {
                // FIXME:
                // info!("parent_guid {} not found, maybe outside of this file", guid);
                "UNK".to_string()
            }
        };
    }
    for s in symbols4export.iter_mut() {
        let symbol_path = recursive_path_of_guid(&guid_to_symbol, &s.borrow().guid);
        s.borrow_mut().symbol_path = symbol_path.clone();
    }
    // longer symbol path at the bottom => parent always higher than children
    symbols4export.sort_by(|a, b| {
        a.borrow().symbol_path.len().cmp(&b.borrow().symbol_path.len())
    });
    Ok(FileASTMarkup {
        file_path: doc.path.clone(),
        file_content: doc.text.as_ref().unwrap().to_string(),
        symbols_sorted_by_path_len: symbols4export.iter().map(|s| {
            s.borrow().clone()
        }).collect(),
    })
}

