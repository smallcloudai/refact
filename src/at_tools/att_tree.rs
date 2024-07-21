use std::collections::HashMap;
use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::Value;
use tracing::{warn, info};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::get_project_paths;
use crate::at_commands::at_tree::{construct_tree_out_of_flat_list_of_paths, print_files_tree_with_budget};
use crate::at_tools::att_file::real_file_path_candidate;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, paths_from_anywhere};


pub struct AttTree;

#[async_trait]
impl Tool for AttTree {
    async fn tool_execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let paths_from_anywhere = paths_from_anywhere(ccx.global_context.clone()).await;
        let path_mb = match args.get("path") {
            Some(Value::String(s)) => Some(s),
            Some(v) => return Err(format!("argument `path` is not a string: {:?}", v)),
            None => None,
        };
        
        let tree = match path_mb {
            Some(path) => {
                let candidates = correct_to_nearest_dir_path(ccx.global_context.clone(), path).await;
                let candidate = real_file_path_candidate(ccx, path, &candidates, &get_project_paths(ccx).await).await?;
                let true_path = PathBuf::from(candidate);
                let filtered_paths_from_anywhere = paths_from_anywhere.iter().filter(|f|f.starts_with(&true_path)).cloned().collect();
                construct_tree_out_of_flat_list_of_paths(&filtered_paths_from_anywhere)
            },
            None => construct_tree_out_of_flat_list_of_paths(&paths_from_anywhere)
        };

        let content = print_files_tree_with_budget(ccx.global_context.clone(), tree).await.map_err(|err| {
            warn!("print_files_tree_with_budget err: {}", err);
            err
        })?;
        
        Ok(vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content,
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
            })
        ])
    }
}
