use crate::at_commands::at_commands::AtCommandsContext;
use std::collections::HashMap;
use std::sync::Arc;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;
use tracing::warn;
use crate::call_validation::{ChatUsage, DiffChunk, SubchatParameters};
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

pub async fn partial_edit_tickets_to_chunks(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    tickets: Vec<TicketToApply>,
    params: &SubchatParameters,
    tool_call_id: &String,
    usage: &mut ChatUsage,
) -> Result<Vec<DiffChunk>, (String, Option<String>)> {
    let gcx = ccx_subchat.lock().await.global_context.clone();
    let mut all_chunks = match execute_blocks_of_code_patch(
        ccx_subchat.clone(),
        tickets.clone(),
        &params.subchat_model,
        params.subchat_n_ctx,
        params.subchat_temperature,
        params.subchat_max_new_tokens,
        tool_call_id,
        usage,
    ).await {
        Ok(chunks) => {
            Ok(chunks)
        }
        Err((err, _)) => {
            warn!("cannot patch file, error: {err}");
            warn!("trying a fallback `whole_file_rewrite` prompt");
            execute_whole_file_patch(
                ccx_subchat.clone(),
                tickets,
                &params.subchat_model,
                params.subchat_n_ctx,
                params.subchat_max_new_tokens,
                tool_call_id,
                usage,
            ).await
        }
    }?;
    let mut chunks_for_answers = vec![];
    for chunks in all_chunks.iter_mut() {
        let diffs = if !chunks.is_empty() {
            postprocess_diff_chunks(gcx.clone(), chunks).await
        } else {
            Ok(vec![])
        };
        chunks_for_answers.push(diffs);
    }
    partial_edit_choose_correct_chunk(chunks_for_answers).map_err(|e| (e, None))
}
