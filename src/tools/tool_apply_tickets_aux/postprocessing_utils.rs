use crate::ast::ast_db::doc_defs;
use crate::ast::ast_structs::AstDefinition;
use crate::call_validation::DiffChunk;
use crate::diffs::{apply_diff_chunks_to_text, correct_and_validate_chunks, unwrap_diff_apply_outputs, ApplyDiffResult};
use crate::global_context::GlobalContext;
use crate::tools::tool_apply_tickets_aux::ast_lint::{lint_and_get_error_messages, parse_and_get_error_symbols};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::ast::ast_indexer_thread::{ast_indexer_block_until_finished, ast_indexer_enqueue_files};
use crate::tools::tool_apply_tickets_aux::fs_utils::read_file;


pub fn minimal_common_indent(symbol_lines: &[&str]) -> (usize, usize) {
    let mut common_spaces = vec![];
    let mut common_tabs = vec![];
    for line in symbol_lines.iter().filter(|l| !l.is_empty()) {
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

        let new_indent = if line.is_empty() { "".to_string() } else { " ".repeat(indent_spaces) + &"\t".repeat(indent_tabs) };
        format!("{}{}", new_indent, trimmed_line)
    }).collect()
}

pub async fn does_doc_have_symbol(
    gcx: Arc<ARwLock<GlobalContext>>,
    symbol: &String,
    doc_path: &String,
) -> Result<(Arc<AstDefinition>, Vec<Arc<AstDefinition>>), String> {
    let symbol_parts = symbol.split("::").map(|s| s.to_string()).collect::<Vec<_>>();
    let ast_service = gcx.read().await.ast_service.clone()
        .ok_or("ast_service is absent".to_string())?;
    let ast_index = ast_service.lock().await.ast_index.clone();
    ast_indexer_enqueue_files(ast_service.clone(), &vec![doc_path.clone()], true).await;
    ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;
    let doc_syms = doc_defs(ast_index, doc_path).await;
    let filtered_syms = doc_syms.iter().filter(|s| s.official_path.ends_with(&symbol_parts)).cloned().collect::<Vec<_>>();
    match filtered_syms.len() {
        0 => Err(format!("symbol '{}' not found in file '{}'", symbol, doc_path)),
        1 => Ok((filtered_syms[0].clone(), doc_syms)),
        _ => Err(format!("cannot locate symbol {}: multiple symbols found with this name", symbol)),
    }
}

pub async fn postprocess_diff_chunks(
    gcx: Arc<ARwLock<GlobalContext>>,
    chunks: &mut Vec<DiffChunk>,
) -> Result<Vec<DiffChunk>, String> {
    if chunks.is_empty() {
        return Err("No diff output, you might have written the same code again.".to_string());
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
            String::new()
        } else {
            match read_file(gcx.clone(), path.to_string_lossy().to_string()).await {
                Ok(text) => text.file_content,
                Err(err) => {
                    let message = format!("Error reading file: {:?}", err);
                    return Err(message);
                }
            }
        };
        let (results, outputs) = apply_diff_chunks_to_text(
            &text_before,
            chunks.iter().enumerate().collect::<Vec<_>>(),
            vec![],
            1,
        );
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());
        let all_applied = outputs_unwrapped.iter().all(|x| x.applied);
        if !all_applied {
            let mut message = format!("Couldn't apply the generated diff, the chunks for the file `{file_name}` are broken:\n").to_string();
            for detail in outputs_unwrapped.iter().filter_map(|x| x.detail.clone()) {
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
            &text_after,
        ).await {
            Ok(symbols) => symbols,
            Err(err) => {
                warn!("Error getting symbols from file: {:?}, skipping ast assessment", err);
                continue;
            }
        };
        if before_error_symbols.len() < after_error_symbols.len() {
            // TODO: return those warnings from the patch along the changed file 
            warn!(
                "AST assessment has failed: the generated diff had introduced errors into the file `{:?}`: {} before errs < {} after errs",
                path, before_error_symbols.len(), after_error_symbols.len()
            );
        }

        let before_lint_errors = lint_and_get_error_messages(
            &path,
            &text_after,
        );
        let after_lint_errors = lint_and_get_error_messages(
            &path,
            &text_after,
        );
        if before_lint_errors.len() < after_lint_errors.len() {
            // TODO: return those warnings from the patch along the changed file 
            warn!(
                "Linting has failed: the generated diff had introduced lint issues into the file `{:?}`: {} before errs < {} after errs",
                path, before_lint_errors.len(), after_lint_errors.len()
            );
        }
    }
    Ok(chunks.to_vec())
}


pub async fn fill_out_already_applied_status(
    gcx: Arc<ARwLock<GlobalContext>>,
    application_results: &mut Vec<ApplyDiffResult>,
) {
    for r in application_results.iter_mut() {
        let file_text_after = match &r.file_text {
            Some(text) => text,
            None => {
                continue;
            }
        };
        let filename = r.file_name_edit.clone()
            .unwrap_or(r.file_name_add.clone()
                .unwrap_or(r.file_name_delete.clone()
                    .unwrap_or(String::from(""))
                ));
        if !filename.is_empty() {
            if let Some(file_text_before) = read_file(gcx.clone(), filename.clone()).await.ok() {
                r.already_applied = file_text_before.file_content == *file_text_after;
            };
        }
    }
}

