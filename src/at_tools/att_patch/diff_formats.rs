use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex as AMutex;
use ropey::Rope;
use tracing::warn;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::att_patch::ast_interaction::parse_and_get_error_symbols;
use crate::at_tools::att_patch::tool::DefaultToolPatch;
use crate::diffs::{apply_diff_chunks_other_to_files, apply_diff_chunks_to_text, can_apply_diff_chunks_other, fuzzy_results_into_state_vector};
use crate::files_in_workspace::read_file_from_disk;


pub async fn parse_diff_chunks_from_message(
    ccx: Arc<AMutex<AtCommandsContext>>,
    message: &String,
) -> Result<String, String> {
    let chunks = match DefaultToolPatch::parse_message(message).await {
        Ok(chunks) => chunks,
        Err(err) => {
            return Err(format!("Error while diff parsing: {:?}", err));
        }
    };

    if chunks.is_empty() {
        return Err("No diff chunks were found".to_string());
    }

    let gcx = ccx.lock().await.global_context.clone();
    let maybe_ast_module = gcx.read().await.ast_module.clone();
    for chunk in chunks.iter() {
        let path = PathBuf::from(&chunk.file_name);
        
        let can_apply_other_raw = can_apply_diff_chunks_other(
            &vec![chunk.clone()], &vec![false], &vec![true]
        );
        let can_apply = fuzzy_results_into_state_vector(
            &can_apply_other_raw, 1).iter().map(|x| *x == 0 || *x == 1
        ).all(|x| x);
        if !can_apply {
            warn!("Couldn't apply the generated diff, the following chunk is broken:\n{:?}", chunk);
            return Err("Couldn't apply the generated diff, probably it's broken".to_string());
        }
        // TODO: temporary
        if chunk.file_action != "edit" {
            continue;
        }

        let text_before = match read_file_from_disk(&path).await {
            Ok(text) => text,
            Err(err) => {
                let message = format!("Error reading file: {:?}", err);
                return Err(message);
            }
        };
        let (text_after, fuzzy_results) = apply_diff_chunks_to_text(
            &text_before.to_string(),
            vec![(0, chunk)],
            vec![],
            1,
        );
        let state = fuzzy_results_into_state_vector(&fuzzy_results, 1);
        if state.iter().any(|x| *x != 1) {
        }
        
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
                        "AST assessment has failed: the generated diff had introduced errors into the file `{:?}`: {} before errs < {} after errs", 
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
