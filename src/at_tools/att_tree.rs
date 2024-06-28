use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::parameter_repair_candidates;
use crate::at_commands::at_tree::{CONTEXT_SIZE_LIMIT, SYMBOLS_PER_TOKEN, tree_from_path};
use crate::at_tools::at_tools::AtTool;
use crate::call_validation::{ChatMessage, ContextEnum};

pub struct AttTree;

#[async_trait]
impl AtTool for AttTree {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let paths = match args.get("path") {
            Some(Value::String(s)) => {
                let candidates = parameter_repair_candidates(&s, ccx).await;
                if candidates.is_empty() {
                    info!("parameter {:?} is uncorrectable :/", &s);
                    return Err(format!("parameter {:?} is uncorrectable :/", &s));
                }
                vec![crate::files_correction::canonical_path(&candidates.get(0).unwrap().clone())]
            }
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) }
            None => { return Err("argument `path` is missing".to_string()) }
        };

        let tree = match tree_from_path(&paths, CONTEXT_SIZE_LIMIT * SYMBOLS_PER_TOKEN as usize) {
            Ok(tree) => tree,
            Err(err) => {
                info!("{}", err);
                return Err(err);
            }
        };

        Ok(vec![
            ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: tree,
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
            })
        ])
    }
}
