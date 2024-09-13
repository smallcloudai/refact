use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};


pub struct ToolAstDefinition;

#[async_trait]
impl Tool for ToolAstDefinition {
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
            let file_paths = defs.iter().map(|x| x.cpath.clone()).collect::<Vec<_>>();
            let short_file_paths = crate::files_correction::shortify_paths(gcx.clone(), file_paths.clone()).await;

            let (messages, tool_message) = if !defs.is_empty() {
                const DEFS_LIMIT: usize = 20;
                let mut tool_message = format!("Definitions found:\n").to_string();
                let messages = defs.iter().zip(short_file_paths.iter()).take(DEFS_LIMIT).map(|(res, short_path)| {
                    tool_message.push_str(&format!(
                        "`{}` at {}:{}-{}\n",
                        res.path(),
                        short_path,
                        res.full_range.start_point.row + 1,
                        res.full_range.end_point.row + 1
                    ));
                    ContextEnum::ContextFile(ContextFile {
                        file_name: short_path.clone(),
                        file_content: "".to_string(),
                        line1: res.full_range.start_point.row + 1,
                        line2: res.full_range.end_point.row + 1,
                        symbols: vec![res.path()],
                        gradient_type: -1,
                        usefulness: 100.0,
                        is_body_important: false,
                    })
                }).collect::<Vec<ContextEnum>>();
                if defs.len() > DEFS_LIMIT {
                    tool_message.push_str(&format!("...and {} more\n", defs.len() - DEFS_LIMIT));
                }
                (messages, tool_message)
            } else {
                corrections = true;
                let fuzzy_matches: Vec<String> = crate::ast::alt_db::definition_paths_fuzzy(ast_index, &symbol).await.into_iter().take(20).collect();
                let mut tool_message = format!(
                    "No definitions with name `{}` found in the workspace, there are definitions with similar names though:\n",
                    symbol
                ).to_string();
                for line in fuzzy_matches {
                    tool_message.push_str(&format!("{}\n", line));
                }
                (vec![], tool_message)
            };

            let mut result_messages = messages;
            result_messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: tool_message.clone(),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            }));
            Ok((corrections, result_messages))
        } else {
            Err("attempt to use @definition with no ast turned on".to_string())
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
