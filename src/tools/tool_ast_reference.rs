use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};

pub struct ToolAstReference;

#[async_trait]
impl Tool for ToolAstReference {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut corrections = false;
        let symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `symbol` is not a string: {:?}", v)),
            None => return Err("argument `symbol` is missing".to_string()),
        };

        let skeleton = match args.get("skeleton") {
            Some(Value::Bool(s)) => *s,
            Some(Value::String(s)) => {
                if s == "true" {
                    true
                } else if s == "false" {
                    false
                } else {
                    return Err(format!("argument `skeleton` is not a bool: {:?}", s));
                }
            }
            Some(v) => return Err(format!("argument `skeleton` is not a bool: {:?}", v)),
            None => false,
        };
        ccx.lock().await.pp_skeleton = skeleton;

        let gcx = ccx.lock().await.global_context.clone();
        let ast_service_opt = gcx.read().await.ast_service.clone();
        if let Some(ast_service) = ast_service_opt {
            let ast_index = ast_service.lock().await.ast_index.clone();
            let defs = crate::ast::alt_db::definitions(ast_index.clone(), &symbol).await;
            let mut all_results = vec![];
            let mut messages = vec![];

            const USAGES_LIMIT: usize = 20;
            const DEFS_LIMIT: usize = 5;

            for (_i, def) in defs.iter().take(DEFS_LIMIT).enumerate() {
                let usages = crate::ast::alt_db::usages(ast_index.clone(), def.path()).await;
                let file_paths = usages.iter().map(|x| x.cpath.clone()).collect::<Vec<_>>();
                let short_file_paths = crate::files_correction::shortify_paths(gcx.clone(), file_paths.clone()).await;

                let def_file_path = vec![def.cpath.clone()];
                let short_def_file_path = crate::files_correction::shortify_paths(gcx.clone(), def_file_path.clone()).await;

                let text = {
                    let usage_count = usages.len();
                    let usage_lines = usages.iter().zip(short_file_paths.iter()).take(USAGES_LIMIT).map(|(u, short_path)| format!("{}:{}", short_path, u.full_range.start_point.row + 1)).collect::<Vec<_>>();
                    let more_usages = if usage_count > USAGES_LIMIT {
                        format!("...and {} more", usage_count - USAGES_LIMIT)
                    } else {
                        String::new()
                    };

                    format!(
                        "For definition `{}` at {}:{}-{} there are {} usages:\n{}\n{}\n",
                        symbol,
                        short_def_file_path.get(0).unwrap_or(&def.path().to_string()),
                        def.full_range.start_point.row + 1,
                        def.full_range.end_point.row + 1,
                        usage_count,
                        usage_lines.join("\n"),
                        more_usages
                    )
                };
                messages.push(text);

                for (res, short_path) in usages.iter().zip(short_file_paths.iter()).take(USAGES_LIMIT) {
                    all_results.push(ContextFile {
                        file_name: short_path.clone(),
                        file_content: "".to_string(),
                        line1: res.full_range.start_point.row + 1,
                        line2: res.full_range.end_point.row + 1,
                        symbols: vec![res.path()],
                        gradient_type: -1,
                        usefulness: 100.0,
                        is_body_important: false
                    });
                }
            }

            if defs.len() > DEFS_LIMIT {
                messages.push(format!("There are {} more symbol definitions that match the query, skipped", defs.len() - DEFS_LIMIT));
            }

            if defs.is_empty() {
                corrections = true;
                let fuzzy_matches: Vec<String> = crate::ast::alt_db::definition_paths_fuzzy(ast_index, &symbol).await.into_iter().take(20).collect();
                let mut tool_message = format!(
                    "No definitions with name `{}` found in the workspace, there are definitions with similar names though:\n",
                    symbol
                ).to_string();
                for line in fuzzy_matches {
                    tool_message.push_str(&format!("{}\n", line));
                }
                messages.push(tool_message);
            }

            let mut result_messages = all_results.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>();
            result_messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: messages.join("\n"),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            }));
            Ok((corrections, result_messages))
        } else {
            Err("attempt to use @reference with no ast turned on".to_string())
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
