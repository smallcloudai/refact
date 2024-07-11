use std::path::PathBuf;

use ropey::Rope;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::ast_interaction::parse_and_get_error_symbols;
use crate::at_tools::att_patch::tool::DefaultToolPatch;
use crate::diffs::apply_diff_chunks_to_text;
use crate::files_in_workspace::read_file_from_disk;

pub async fn parse_diff_chunks_from_message(
    ccx: &mut AtCommandsContext,
    message: &String,
) -> Result<String, String> {
    let chunks = match DefaultToolPatch::parse_message(message).await {
        Ok(chunks) => chunks,
        Err(err) => {
            return Err(format!("Error parsing diff: {:?}", err));
        }
    };

    if chunks.is_empty() {
        return Err("No diff chunks were found".to_string());
    }

    let gx = ccx.global_context.clone();
    let maybe_ast_module = gx.read().await.ast_module.clone();
    for chunk in chunks.iter() {
        let path = PathBuf::from(&chunk.file_name);
        let text_before = match read_file_from_disk(&path).await {
            Ok(text) => text,
            Err(err) => {
                let message = format!("Error reading file: {:?}, skipping ast assessment", err);
                return Err(message);
            }
        };
        let (text_after, _) = apply_diff_chunks_to_text(
            &text_before.to_string(),
            chunks.iter().enumerate().collect::<Vec<_>>(),
            vec![],
            1,
        );
        match &maybe_ast_module {
            Some(ast_module) => {
                let before_error_symbols = match parse_and_get_error_symbols(
                    ast_module.clone(),
                    &path,
                    &text_before,
                ).await {
                    Ok(symbols) => symbols,
                    Err(err) => {
                        warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                        continue;
                    }
                };
                let after_error_symbols = match parse_and_get_error_symbols(
                    ast_module.clone(),
                    &path,
                    &Rope::from_str(&text_after),
                ).await {
                    Ok(symbols) => symbols,
                    Err(err) => {
                        warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                        continue;
                    }
                };
                if before_error_symbols.len() < after_error_symbols.len() {
                    let message = format!(
                        "Ast assessment failed: the diff introduced errors into the file {:?}: {}errs > {}errs", 
                        path, before_error_symbols.len(), after_error_symbols.len()
                    );
                    return Err(message);
                }
            }
            None => {
                warn!("AST module is disabled, the diff assessment is skipping");
            }
        }
    }

    match serde_json::to_string_pretty(&chunks) {
        Ok(json_chunks) => Ok(json_chunks),
        Err(err) => Err(format!("Error diff chunks serializing: {:?}", err))
    }
}
