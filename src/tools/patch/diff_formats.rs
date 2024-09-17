use std::sync::Arc;
use std::path::PathBuf;
use hashbrown::HashMap;
use tokio::sync::Mutex as AMutex;
use ropey::Rope;
use tracing::warn;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::patch::ast_interaction::{lint_and_get_error_messages, parse_and_get_error_symbols};
use crate::call_validation::DiffChunk;
use crate::diffs::{apply_diff_chunks_to_text, correct_and_validate_chunks, unwrap_diff_apply_outputs};
use crate::files_in_workspace::read_file_from_disk;
use crate::privacy::load_privacy_if_needed;


pub async fn postprocess_diff_chunks_from_message(
    ccx: Arc<AMutex<AtCommandsContext>>,
    chunks: &mut Vec<DiffChunk>,
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();

    if chunks.is_empty() {
        return Err("No diff chunks were found".to_string());
    }

    correct_and_validate_chunks(gcx.clone(), chunks).await?;
    let mut chunks_per_files = HashMap::new();
    for chunk in chunks.iter() {
        chunks_per_files.entry(chunk.file_name.clone()).or_insert(vec![]).push(chunk.clone());
    }
    for (file_name, chunks) in chunks_per_files {
        let path = PathBuf::from(&file_name);
        let action = chunks
            .first()
            .map(|x| x.file_action.clone())
            .expect("chunks should have at least one element");
        if (action == "add" || action == "remove" || action == "rename") && chunks.len() > 1 {
            warn!("The file `{:?}` has multiple `add` or `remove` or `rename` diff chunks, it's not supported now", path);
            return Err(format!("The file `{:?}` has multiple `add` or `remove` or `rename` diff chunks, it's not supported now", path));
        }

        let text_before = if action == "add" {
            Rope::new()
        } else {
            match read_file_from_disk(load_privacy_if_needed(gcx.clone()).await, &path).await {
                Ok(text) => text,
                Err(err) => {
                    let message = format!("Error reading file: {:?}", err);
                    return Err(message);
                }
            }
        };
        let (results, outputs) = apply_diff_chunks_to_text(
            &text_before.to_string(),
            chunks.iter().enumerate().collect::<Vec<_>>(),
            vec![],
            1
        );
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());
        let all_applied = outputs_unwrapped.iter().all(|x|x.applied);
        if !all_applied {
            let mut message = format!("Couldn't apply the generated diff, the chunks for the file `{file_name}` are broken:\n").to_string();
            for detail in outputs_unwrapped.iter().filter_map(|x| x.detail.clone())  {
                message.push_str(&format!("{detail}\n"));
            }
            warn!(message);
            return Err(message);
        }
        if results.is_empty() {
            warn!("No apply results were found for the filename:\n{:?}", file_name);
            return Err(format!("No apply results were found for the filename:\n{:?}", file_name));
        }

        let text_after = if let Some(file_text) = results.first().map(|x| x.file_text.clone()).flatten() {
            file_text
        } else {
            // those chunks could miss the text_after, so we just skip them
            if action == "remove" || action == "rename" {
                continue;
            }
            warn!("Diff application error: text_after is missing for the filename:\n{:?}", file_name);
            return Err(format!("Diff application error: text_after is missing for the filename:\n{:?}", file_name));
        };
        let before_error_symbols = match parse_and_get_error_symbols(
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

        let before_lint_errors = lint_and_get_error_messages(
            &path,
            &Rope::from_str(&text_after),
        );
        let after_lint_errors = lint_and_get_error_messages(
            &path,
            &Rope::from_str(&text_after),
        );
        if before_lint_errors.len() < after_lint_errors.len() {
            let message = format!(
                "Linting has failed: the generated diff had introduced lint issues into the file `{:?}`: {} before errs < {} after errs",
                path, before_lint_errors.len(), after_lint_errors.len()
            );
            return Err(message);
        }
    }

    serde_json::to_string_pretty(&chunks)
        .map_err(|e| format!("Error diff chunks serializing: {:?}", e))
}
