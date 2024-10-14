use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::ast::ast_structs::AstDB;
use crate::ast::ast_db::fetch_counters;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};


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
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `symbol` is not a string: {:?}", v)),
            None => return Err("argument `symbol` is missing".to_string()),
        };

        symbol = symbol.replace('.', "::");

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

            crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;
            let defs = crate::ast::ast_db::definitions(ast_index.clone(), &symbol).await;

            let file_paths = defs.iter().map(|x| x.cpath.clone()).collect::<Vec<_>>();
            let short_file_paths = crate::files_correction::shortify_paths(gcx.clone(), &file_paths).await;

            let (messages, tool_message) = if !defs.is_empty() {
                const DEFS_LIMIT: usize = 20;
                let mut tool_message = format!("Definitions found:\n").to_string();
                let messages = defs.iter().zip(short_file_paths.iter()).take(DEFS_LIMIT).map(|(res, short_path)| {
                    tool_message.push_str(&format!(
                        "{} defined at {}:{}-{}\n",
                        res.path_drop0(),
                        short_path,
                        res.full_range.start_point.row + 1,
                        res.full_range.end_point.row + 1
                    ));
                    ContextEnum::ContextFile(ContextFile {
                        file_name: res.cpath.clone(),
                        file_content: "".to_string(),
                        line1: res.full_range.start_point.row + 1,
                        line2: res.full_range.end_point.row + 1,
                        symbols: vec![res.path_drop0()],
                        gradient_type: -1,
                        usefulness: 100.0,
                    })
                }).collect::<Vec<ContextEnum>>();
                if defs.len() > DEFS_LIMIT {
                    tool_message.push_str(&format!("...and {} more\n", defs.len() - DEFS_LIMIT));
                }
                (messages, tool_message)
            } else {
                corrections = true;
                let tool_message = there_are_definitions_with_similar_names_though(ast_index, &symbol).await;
                (vec![], tool_message)
            };

            let mut result_messages = messages;
            result_messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(tool_message),
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

pub async fn there_are_definitions_with_similar_names_though(
    ast_index: Arc<AMutex<AstDB>>,
    symbol: &str,
) -> String {
    let fuzzy_matches: Vec<String> = crate::ast::ast_db::definition_paths_fuzzy(ast_index.clone(), symbol, 20, 5000)
        .await;

    let tool_message = if fuzzy_matches.is_empty() {
        let counters = fetch_counters(ast_index).await;
        format!("No definitions with name `{}` found in the workspace, and no similar names were found among {} definitions in the AST tree.\n", symbol, counters.counter_defs)
    } else {
        let mut msg = format!(
            "No definitions with name `{}` found in the workspace, there are definitions with similar names though:\n",
            symbol
        );
        for line in fuzzy_matches {
            msg.push_str(&format!("{}\n", line));
        }
        msg
    };

    tool_message
}
