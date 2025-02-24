use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::tools::tool_ast_definition::there_are_definitions_with_similar_names_though;
use crate::tools::tool_cat::parse_skeleton_from_args;

pub struct ToolAstReference;

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
        let mut symbol = match args.get("symbol") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `symbol` is not a string: {:?}", v)),
            None => return Err("argument `symbol` is missing".to_string()),
        };

        symbol = symbol.replace('.', "::");

        let skeleton = parse_skeleton_from_args(args)?;
        ccx.lock().await.pp_skeleton = skeleton;

        let gcx = ccx.lock().await.global_context.clone();
        let ast_service_opt = gcx.read().await.ast_service.clone();
        if let Some(ast_service) = ast_service_opt {
            let ast_index = ast_service.lock().await.ast_index.clone();

            crate::ast::ast_indexer_thread::ast_indexer_block_until_finished(ast_service.clone(), 20_000, true).await;
            let defs = crate::ast::ast_db::definitions(ast_index.clone(), &symbol).await;

            let mut all_results = vec![];
            let mut messages = vec![];

            const USAGES_LIMIT: usize = 20;
            const DEFS_LIMIT: usize = 5;

            for (_i, def) in defs.iter().take(DEFS_LIMIT).enumerate() {
                let usedin_and_uline = crate::ast::ast_db::usages(ast_index.clone(), def.path(), 100).await;
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
                messages.push(text);

                for (usedin, uline) in usedin_and_uline.iter().take(USAGES_LIMIT) {
                    all_results.push(ContextFile {
                        file_name: usedin.cpath.clone(),
                        file_content: "".to_string(),
                        line1: *uline,
                        line2: *uline,
                        symbols: vec![usedin.path()],
                        gradient_type: -1,
                        usefulness: 100.0,
                    });
                }
            }

            if defs.len() > DEFS_LIMIT {
                messages.push(format!("There are {} more symbol definitions that match the query, skipped", defs.len() - DEFS_LIMIT));
            }

            if defs.is_empty() {
                corrections = true;
                let fuzzy_message = there_are_definitions_with_similar_names_though(ast_index, &symbol).await;
                messages.push(fuzzy_message);
            }

            let mut result_messages = all_results.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>();
            result_messages.push(ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(messages.join("\n")),
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
