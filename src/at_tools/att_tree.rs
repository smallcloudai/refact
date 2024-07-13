use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::at_file_repair_candidates;
use crate::at_commands::at_tree::{construct_tree_out_of_flat_list_of_paths, print_files_tree_with_budget};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::files_correction::{canonical_path, paths_from_anywhere};

pub struct AttTree;

#[async_trait]
impl Tool for AttTree {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let paths_from_anywhere = paths_from_anywhere(ccx.global_context.clone()).await;
        let tree = match args.get("path") {
            Some(Value::String(s)) => {
                let base_path = canonical_path(&s);
                let filtered_paths_from_anywhere = paths_from_anywhere
                    .iter()
                    .filter(|file| file.starts_with(&base_path))
                    .cloned()
                    .collect();
                construct_tree_out_of_flat_list_of_paths(&filtered_paths_from_anywhere)
            }
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) }
            None => {
                construct_tree_out_of_flat_list_of_paths(&paths_from_anywhere)
            }
        };

        tracing::info!("\n{:#?}", tree);
        let content = match print_files_tree_with_budget(
            ccx.global_context.clone(), tree
        ).await {
            Ok(content) => content,
            Err(err) => {
                info!("{}", err);
                return Err(err);
            }
        };

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
