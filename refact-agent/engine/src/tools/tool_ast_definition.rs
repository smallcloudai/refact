use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::ast::ast_structs::AstDB;
use crate::ast::ast_db::fetch_counters;
use crate::custom_error::trace_and_default;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::postprocessing::pp_command_output::OutputFilter;

pub struct ToolAstDefinition {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolAstDefinition {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut corrections = false;
        let symbols_str = match args.get("symbols") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `symbols` is not a string: {:?}", v)),
            None => return Err("argument `symbols` is missing".to_string()),
        };

        let symbols: Vec<String> = symbols_str
            .split(',')
            .map(|s| s.trim().replace('.', "::"))
            .filter(|s| !s.is_empty())
            .collect();

        if symbols.is_empty() {
            return Err("No valid symbols provided".to_string());
        }

        let gcx = ccx.lock().await.global_context.clone();
        let ast_service_opt = gcx.read().await.ast_service.clone();
        if let Some(ast_service) = ast_service_opt {
            let ast_index = ast_service.lock().await.ast_index.clone();

            crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;

            let mut all_messages = Vec::new();
            let mut all_context_files = Vec::new();

            for symbol in symbols {
                let defs = crate::ast::ast_db::definitions(ast_index.clone(), &symbol).unwrap_or_default();

                let file_paths = defs.iter().map(|x| x.cpath.clone()).collect::<Vec<_>>();
                let short_file_paths = crate::files_correction::shortify_paths(gcx.clone(), &file_paths).await;

                if !defs.is_empty() {
                    const DEFS_LIMIT: usize = 20;
                    let mut tool_message = format!("Definitions for `{}`:\n", symbol).to_string();
                    let context_files: Vec<ContextEnum> = defs.iter().zip(short_file_paths.iter()).take(DEFS_LIMIT).map(|(res, short_path)| {
                        tool_message.push_str(&format!(
                            "{} defined at {}:{}-{}\n",
                            res.path_drop0(),
                            short_path,
                            res.full_line1(),
                            res.full_line2()
                        ));
                        ContextEnum::ContextFile(ContextFile {
                            file_name: res.cpath.clone(),
                            file_content: "".to_string(),
                            line1: res.full_line1(),
                            line2: res.full_line2(),
                            symbols: vec![res.path_drop0()],
                            gradient_type: 5,
                            usefulness: 100.0,
                            skip_pp: false,
                        })
                    }).collect();

                    if defs.len() > DEFS_LIMIT {
                        tool_message.push_str(&format!("...and {} more\n", defs.len() - DEFS_LIMIT));
                    }

                    all_messages.push(tool_message);
                    all_context_files.extend(context_files);
                } else {
                    corrections = true;
                    let tool_message = there_are_definitions_with_similar_names_though(ast_index.clone(), &symbol).await;
                    all_messages.push(format!("For symbol `{}`:\n{}", symbol, tool_message));
                }
            }

            let combined_message = all_messages.join("\n");
            all_context_files.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(combined_message),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                output_filter: Some(OutputFilter::no_limits()), // Already compressed internally
                ..Default::default()
            }));

            Ok((corrections, all_context_files))
        } else {
            Err("attempt to use search_symbol_definition with no ast turned on".to_string())
        }
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_symbol_definition".to_string(),
            display_name: "Definition".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Find definition of a symbol in the project using AST".to_string(),
            parameters: vec![
                ToolParam {
                    name: "symbols".to_string(),
                    description: "Comma-separated list of symbols to search for (functions, methods, classes, type aliases). No spaces allowed in symbol names.".to_string(),
                    param_type: "string".to_string(),
                },
            ],
            parameters_required: vec!["symbols".to_string()],
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

pub async fn there_are_definitions_with_similar_names_though(
    ast_index: Arc<AstDB>,
    symbol: &str,
) -> String {
    let fuzzy_matches: Vec<String> = crate::ast::ast_db::definition_paths_fuzzy(ast_index.clone(), symbol, 20, 5000)
        .await
        .unwrap_or_else(trace_and_default);

    let tool_message = if fuzzy_matches.is_empty() {
        let counters = fetch_counters(ast_index).unwrap_or_else(trace_and_default);
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
