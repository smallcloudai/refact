use async_trait::async_trait;
use std::sync::Arc;

use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{AtParamFilePath, at_file_repair_candidates};
use crate::at_commands::at_workspace::execute_at_workspace;
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::call_validation::{ContextEnum, ContextFile};


pub struct AtFileSearch {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtFileSearch {
    pub fn new() -> Self {
        AtFileSearch {
            name: "@file-search".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

pub fn text_on_clip(query: &String, file_path: &String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query.clone();
    }
    return format!("performed vecdb search in file: {}\nthe result is attached below", file_path);
}


pub async fn execute_at_file_search(
    ccx: &mut AtCommandsContext,
    file_path: &String,
    query: &String,
    from_tool_call: bool,
) -> Result<Vec<ContextFile>, String> {
    let fuzzy = !from_tool_call;
    let candidates = at_file_repair_candidates(file_path, ccx, fuzzy).await;
    if candidates.is_empty() {
        info!("parameter {:?} is uncorrectable :/", file_path);
        return Err(format!("parameter {:?} is uncorrectable :/", file_path));
    }
    let file_path = candidates.get(0).unwrap().clone();
    let vecdb_scope_filter = format!("(file_path = \"{}\")", file_path);
    let vector_of_context_file = execute_at_workspace(ccx, query, Some(vecdb_scope_filter)).await?;

    Ok(vector_of_context_file)
}

#[async_trait]
impl AtCommand for AtFileSearch {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("missing file path".to_string());
                args.clear();
                return Err("missing file path".to_string());
            }
        };
        correct_at_arg(ccx, self.params[0].clone(), &mut file_path).await;
        args.clear();
        args.push(file_path.clone());

        if !file_path.ok {
            return Err(format!("file_path is incorrect: {:?}. Reason: {:?}", file_path.text, file_path.reason));
        }

        // note: skipping file_path which is first argument
        let query = args.iter().skip(1).map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");

        let vector_of_context_file = execute_at_file_search(ccx, &file_path.text, &query, false).await?;
        let text = text_on_clip(&query, &file_path.text, false);

        Ok((vec_context_file_to_context_tools(vector_of_context_file), text))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}