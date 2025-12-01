use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::custom_error::trace_and_default;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::tools::tool_ast_definition::there_are_definitions_with_similar_names_though;

pub struct ToolAstReference {
    pub config_path: String,
}

#[async_trait]
impl Tool for ToolAstReference {
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

            let mut all_results = vec![];
            let mut all_messages = vec![];

            const USAGES_LIMIT: usize = 20;
            const DEFS_LIMIT: usize = 5;

            for symbol in symbols {
                let defs = crate::ast::ast_db::definitions(ast_index.clone(), &symbol).unwrap_or_else(trace_and_default);
                let mut symbol_messages = vec![];

                if !defs.is_empty() {
                    for (_i, def) in defs.iter().take(DEFS_LIMIT).enumerate() {
                        let usedin_and_uline = crate::ast::ast_db::usages(ast_index.clone(), def.path(), 100).unwrap_or_else(trace_and_default);
                        let file_paths = usedin_and_uline.iter().map(|(usedin, _)| usedin.cpath.clone()).collect::<Vec<_>>();
                        let short_file_paths = crate::files_correction::shortify_paths(gcx.clone(), &file_paths).await;

                        let def_file_path = vec![def.cpath.clone()];
                        let short_def_file_path = crate::files_correction::shortify_paths(gcx.clone(), &def_file_path).await;

                        let text = {
                            let usage_count = usedin_and_uline.len();
                            let mut usage_lines = Vec::new();
                            for ((_usedin, uline), short_path) in usedin_and_uline.iter().zip(short_file_paths.iter()).take(USAGES_LIMIT) {
                                usage_lines.push(format!("{}:{}", short_path, uline));
                            }
                            let more_usages = if usage_count > USAGES_LIMIT {
                                format!("...and {} more", usage_count - USAGES_LIMIT)
                            } else {
                                String::new()
                            };

                            format!(
                                "For {} defined at {}:{}-{} there are {} usages:\n{}\n{}\n",
                                def.path_drop0(),
                                short_def_file_path.get(0).unwrap_or(&def.path().to_string()),
                                def.full_line1(),
                                def.full_line2(),
                                usage_count,
                                usage_lines.join("\n"),
                                more_usages
                            )
                        };
                        symbol_messages.push(text);

                        for (usedin, uline) in usedin_and_uline.iter().take(USAGES_LIMIT) {
                            all_results.push(ContextFile {
                                file_name: usedin.cpath.clone(),
                                file_content: "".to_string(),
                                line1: *uline,
                                line2: *uline,
                                symbols: vec![usedin.path()],
                                gradient_type: 4,
                                usefulness: 100.0,
                                skip_pp: false,
                            });
                        }
                    }

                    if defs.len() > DEFS_LIMIT {
                        symbol_messages.push(format!("There are {} more symbol definitions that match the query, skipped", defs.len() - DEFS_LIMIT));
                    }
                } else {
                    corrections = true;
                    let fuzzy_message = there_are_definitions_with_similar_names_though(ast_index.clone(), &symbol).await;
                    symbol_messages.push(format!("For symbol `{}`:\n{}", symbol, fuzzy_message));
                }

                all_messages.push(format!("Results for symbol `{}`:\n{}", symbol, symbol_messages.join("\n")));
            }

            let mut result_messages = all_results.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>();
            result_messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(all_messages.join("\n\n")),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            }));
            Ok((corrections, result_messages))
        } else {
            Err("attempt to use search_symbol_usages with no ast turned on".to_string())
        }
    }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_symbol_usages".to_string(),
            display_name: "References".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Find usages of a symbol within a project using AST".to_string(),
            parameters: vec![
                ToolParam {
                    name: "symbols".to_string(),
                    description: "Comma-separated list of symbols to search for (functions, methods, classes, type aliases). No spaces allowed in symbol names.".to_string(),
                    param_type: "string".to_string(),
                }
            ],
            parameters_required: vec!["symbols".to_string()],
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
