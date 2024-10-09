use crate::at_commands::at_commands::AtCommandsContext;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::call_validation::{ChatUsage, DiffChunk, SubchatParameters};
use crate::global_context::GlobalContext;
use crate::tools::tool_patch_aux::fs_utils::read_file;
use crate::tools::tool_patch_aux::model_based_edit::model_execution::{execute_blocks_of_code_patch, execute_whole_file_patch};
use crate::tools::tool_patch_aux::postprocessing_utils::postprocess_diff_chunks;
use crate::tools::tool_patch_aux::tickets_parsing::TicketToApply;

fn partial_edit_choose_correct_chunk(chunks: Vec<Result<Vec<DiffChunk>, String>>) -> Result<Vec<DiffChunk>, String> {
    let errors = chunks
        .iter()
        .filter(|res| res.is_err())
        .map(|res| res.clone().unwrap_err())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        warn!("There is a list of errors for some generated diffs");
        for err in errors.iter() {
            warn!("{err}");
        }
    }
    if chunks.iter().all(|res| res.is_err()) {
        let mut err_message = "No valid chunks were generated, reasons are:\n".to_string();
        for err in errors.iter().unique() {
            err_message.push_str(format!("- {err}\n").as_str());
        }
        err_message.push_str("Try to call `patch` one more time to generate a correct diff");
        return Err(err_message);
    }

    let non_error_chunks = chunks
        .iter()
        .filter_map(|res| res.as_ref().ok())
        .cloned()
        .collect::<Vec<_>>();
    warn!("{} diff were parsed successfully", non_error_chunks.len());

    // return the most common chunk
    let mut chunks_freq = HashMap::new();
    for chunk in non_error_chunks.iter() {
        *chunks_freq.entry(chunk).or_insert(0) += 1;
    }
    let max_repeats = chunks_freq.iter().max_by_key(|(_, k)| *k).unwrap().1.clone();
    let chunks_max_repeats = chunks_freq
        .iter()
        .filter(|(_, v)| **v == max_repeats)
        .map(|x| *x.0)
        .collect::<Vec<_>>();
    Ok(
        (*chunks_max_repeats.iter().max().expect("There is no max repeats"))
            .clone()
    )
}

async fn is_file_too_big(
    gcx: Arc<ARwLock<GlobalContext>>,
    tickets: Vec<TicketToApply>,
) -> Result<bool, String> {
    let filename = PathBuf::from(
        tickets
            .get(0)
            .expect("no tickets provided")
            .filename_before
            .clone()
    );
    let context_file = read_file(gcx, filename.to_string_lossy().to_string())
        .await
        .map_err(|e| format!("cannot read file to modify: {:?}.\nError: {e}", filename))?;

    // more than 15Kb is too big
    Ok(context_file.file_content.len() > 15_000)
}

pub async fn partial_edit_tickets_to_chunks(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    tickets: Vec<TicketToApply>,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<DiffChunk>, (String, Option<String>)> {
    let gcx = ccx_subchat.lock().await.global_context.clone();
    let mut all_chunks = match is_file_too_big(gcx.clone(), tickets.clone()).await {
        Ok(too_big) => {
            if too_big {
                warn!("given file is too big, using execute_blocks_of_code_patch");
                execute_blocks_of_code_patch(
                    ccx_subchat.clone(),
                    tickets,
                    &params.subchat_model,
                    params.subchat_n_ctx,
                    params.subchat_temperature,
                    params.subchat_max_new_tokens,
                    tool_call_id,
                    usage,
                ).await?
            } else {
                warn!("using execute_whole_file_patch");
                execute_whole_file_patch(
                    ccx_subchat.clone(),
                    tickets,
                    &params.subchat_model,
                    params.subchat_n_ctx,
                    params.subchat_temperature,
                    params.subchat_max_new_tokens,
                    tool_call_id,
                    usage,
                ).await?
            }
        }
        Err(err) => {
            return Err((err, None));
        }
    };

    let mut chunks_for_answers = vec![];
    for chunks in all_chunks.iter_mut() {
        let diffs = postprocess_diff_chunks(gcx.clone(), chunks).await;
        chunks_for_answers.push(diffs);
    }
    partial_edit_choose_correct_chunk(chunks_for_answers).map_err(|e| (e, None))
}
