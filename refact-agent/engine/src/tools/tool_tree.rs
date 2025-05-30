use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;
use tracing::warn;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::return_one_candidate_or_a_good_error;
use crate::at_commands::at_tree::{print_files_tree_with_budget, TreeNode};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::files_correction::{correct_to_nearest_dir_path, correct_to_nearest_filename, get_project_dirs, paths_from_anywhere};
use crate::files_in_workspace::ls_files;


pub struct ToolTree {
    pub config_path: String,
}

fn preformat_path(path: &String) -> String {
    path.trim_end_matches(&['/', '\\'][..]).to_string()
}

#[async_trait]
impl Tool for ToolTree {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "tree".to_string(),
            display_name: "Tree".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Get a files tree with symbols for the project. Use it to get familiar with the project, file names and symbols".to_string(),
            parameters: vec![
                ToolParam {
                    name: "path".to_string(),
                    description: "An absolute path to get files tree for. Do not pass it if you need a full project tree.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "use_ast".to_string(),
                    description: "If true, for each file an array of AST symbols will appear as well as its filename".to_string(),
                    param_type: "boolean".to_string(),
                },
            ],
            parameters_required: vec![],
        }
    }

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

                let project_dirs = get_project_dirs(gcx.clone()).await;
                let candidate = return_one_candidate_or_a_good_error(
                    gcx.clone(), &path, &dir_candidates, &project_dirs, true
                ).await?;
                let true_path = crate::files_correction::canonical_path(candidate);

                let is_within_project_dirs = project_dirs.iter().any(|p| true_path.starts_with(&p));
                if !is_within_project_dirs && !gcx.read().await.cmdline.inside_container {
                    return Err(format!("Cannot execute tree(), '{path}' is not within the project directories."));
                }

                let indexing_everywhere = crate::files_blocklist::reload_indexing_everywhere_if_needed(gcx.clone()).await;
                let paths_in_dir = ls_files(&indexing_everywhere, &true_path, true).unwrap_or(vec![]);

                TreeNode::build(&paths_in_dir)
            },
            None => TreeNode::build(&paths_from_anywhere)
        };

        let content = print_files_tree_with_budget(ccx.clone(), &tree, use_ast).await.map_err(|err| {
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
