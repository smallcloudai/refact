use crate::at_commands::at_file::{context_file_from_file_path, file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::call_validation::ContextFile;
use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

pub async fn read_file(
    gcx: Arc<ARwLock<GlobalContext>>,
    file_path: String,
) -> Result<ContextFile, String> {
    let candidates = file_repair_candidates(gcx.clone(), &file_path, 10, false).await;
    let candidate = return_one_candidate_or_a_good_error(
        gcx.clone(), &file_path, &candidates, &get_project_dirs(gcx.clone()).await, false,
    ).await?;
    context_file_from_file_path(gcx.clone(), candidate).await
}
