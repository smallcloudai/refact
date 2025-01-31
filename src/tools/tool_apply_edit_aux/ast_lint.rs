use std::path::PathBuf;
use rand::distributions::Alphanumeric;
use rand::Rng;
use ropey::Rope;
use crate::ast::linters::lint;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, SymbolInformation};
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::files_in_workspace::Document;

pub async fn parse_and_get_error_symbols(
    path: &PathBuf,
    file_text: &String,
) -> Result<Vec<SymbolInformation>, String> {
    let (mut parser, _language) = match get_ast_parser_by_filename(&path) {
        Ok(x) => x,
        Err(err) => {
            tracing::info!("Error getting parser: {}", err.message);
            return Err(format!("Error getting parser: {}", err.message));
        }
    };

    let symbols: Vec<AstSymbolInstanceArc> = parser.parse(&file_text, path);
    let error_symbols: Vec<SymbolInformation> = symbols
        .into_iter()
        .filter_map(|symbol| {
            let symbol_info = symbol.read().symbol_info_struct();
            if symbol_info.is_error {
                Some(symbol_info)
            } else {
                None
            }
        })
        .collect();

    Ok(error_symbols)
}

pub fn lint_and_get_error_messages(
    path: &PathBuf,
    file_text: &String,
) -> Vec<String> {
    let dummy_filename = PathBuf::from(rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>());
    let new_filename = dummy_filename.with_extension(
        path.extension().unwrap_or_default()
    );
    let doc = Document { doc_path: new_filename.clone(), doc_text: Some(Rope::from_str(file_text)) };
    match lint(&doc) {
        Ok(_) => vec![],
        Err(problems) => problems,
    }
}
