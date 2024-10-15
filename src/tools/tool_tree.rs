use std::any::Any;
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use tracing::warn;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::at_commands::at_tree::{construct_tree_out_of_flat_list_of_paths, print_files_tree_with_budget};
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, correct_to_nearest_filename, get_project_dirs, paths_from_anywhere};
use crate::files_in_workspace::ls_files;


pub struct ToolTree;

fn preformat_path(path: &String) -> String {
    path.trim_end_matches(&['/', '\\'][..]).to_string()
}

#[async_trait]
impl Tool for ToolTree {
    fn as_any(&self) -> &dyn Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let gcx = ccx.lock().await.global_context.clone();
        let paths_from_anywhere = paths_from_anywhere(gcx.clone()).await;

        let path_mb = match args.get("path") {
            Some(Value::String(s)) => Some(preformat_path(s)),
            Some(v) => return Err(format!("argument `path` is not a string: {:?}", v)),
            None => None,
        };
        let use_ast = match args.get("use_ast") {
            Some(Value::Bool(b)) => *b,
            Some(v) => return Err(format!("argument `use_ast` is not a boolean: {:?}", v)),
            None => false,
        };


        let tree = match path_mb {
            Some(path) => {
                let file_candidates = correct_to_nearest_filename(gcx.clone(), &path, false, 10).await;
                let dir_candidates = correct_to_nearest_dir_path(gcx.clone(), &path, false, 10).await;
                if dir_candidates.is_empty() && !file_candidates.is_empty() {
                    return Err("Cannot execute tree() because 'path' provided refers to a file.".to_string());
                }

                let candidate = return_one_candidate_or_a_good_error(
                    gcx.clone(), &path, &dir_candidates, &get_project_dirs(gcx.clone()).await, true
                ).await?;
                let true_path = PathBuf::from(candidate);
                let paths_in_dir = ls_files(&true_path, true).unwrap_or(vec![]);
                construct_tree_out_of_flat_list_of_paths(&paths_in_dir)
            },
            None => construct_tree_out_of_flat_list_of_paths(&paths_from_anywhere)
        };

        let content = print_files_tree_with_budget(ccx.clone(), tree, use_ast).await.map_err(|err| {
            warn!("print_files_tree_with_budget err: {}", err);
            err
        })?;

        Ok((false, vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(content),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            })
        ]))
    }
}
