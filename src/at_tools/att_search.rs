use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::{AtCommandsContext, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{at_file_repair_candidates, get_project_paths};
use crate::at_commands::at_search::{execute_at_search, text_on_clip};
use crate::at_tools::att_file::real_file_path_candidate;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};


pub struct AttSearch;

async fn execute_att_search(ccx: &mut AtCommandsContext, query: &String, scope: &String) -> Result<Vec<ContextFile>, String> {
    fn is_scope_a_file(scope: &String) -> bool {
        PathBuf::from(scope).extension().is_some()
    }
    fn is_scope_a_dir(scope: &String) -> bool {
        let path = PathBuf::from(scope);
        match fs::metadata(&path) {
            Ok(metadata) => metadata.is_dir(),
            Err(_) => false,
        }
    }
    
    return match scope.as_str() {
        "workspace" => {
            Ok(execute_at_search(ccx, &query, None).await?)
        },
        _ if is_scope_a_file(scope) => {
            let candidates = at_file_repair_candidates(scope, ccx, false).await;
            let file_path = real_file_path_candidate(ccx, scope, &candidates, &get_project_paths(ccx).await, false).await?;
            let filter = Some(format!("(file_path = \"{}\")", file_path));
            Ok(execute_at_search(ccx, &query, filter).await?)
        },
        _ if is_scope_a_dir(scope) => {
            let filter = format!("(file_path LIKE '{}%')", scope);
            Ok(execute_at_search(ccx, &query, Some(filter)).await?)
        },
        _ => Err(format!("scope {} is not supported", scope))
    };
}

#[async_trait]
impl Tool for AttSearch {
    async fn tool_execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let query = match args.get("query") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("Missing argument `query` for att_search".to_string())
        };
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` for att_search".to_string())
        };
        let vector_of_context_file = execute_att_search(ccx, &query, &scope).await?;
        println!("att-search: vector_of_context_file={:?}", vector_of_context_file);
        let text = text_on_clip(&query, true);

        let mut results = vec_context_file_to_context_tools(vector_of_context_file);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: text,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
