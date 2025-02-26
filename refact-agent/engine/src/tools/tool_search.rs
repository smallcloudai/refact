use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::info;

use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::at_commands::at_search::execute_at_search;
use crate::tools::scope_utils::create_scope_filter;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};


pub struct ToolSearch;

async fn execute_att_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &String,
    scope: &String,
) -> Result<Vec<ContextFile>, String> {
    let gcx = ccx.lock().await.global_context.clone();
    
    // Use the common function to create a scope filter
    let filter = create_scope_filter(gcx.clone(), scope).await?;

    info!("att-search: filter: {:?}", filter);
    execute_at_search(ccx.clone(), &query, filter).await
}

#[async_trait]
impl Tool for ToolSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query = match args.get("query") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `query` is not a string: {:?}", v)),
            None => return Err("Missing argument `query` in the search() call.".to_string())
        };
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` in the search() call.".to_string())
        };

        let vector_of_context_file = execute_att_search(ccx.clone(), &query, &scope).await?;
        info!("att-search: vector_of_context_file={:?}", vector_of_context_file);

        if vector_of_context_file.is_empty() {
            return Err("Search produced no results, adjust the query or try a different scope.".to_string());
        }

        let mut content = "Records found:\n\n".to_string();
        let mut file_results_to_reqs: HashMap<String, Vec<&ContextFile>> = HashMap::new();
        vector_of_context_file.iter().for_each(|rec| {
            file_results_to_reqs.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
        });
        let mut used_files: HashSet<String> = HashSet::new();
        for rec in vector_of_context_file.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
            if !used_files.contains(&rec.file_name) {
                content.push_str(&format!("{}:\n", rec.file_name.clone()));
                let file_recs = file_results_to_reqs.get(&rec.file_name).unwrap();
                for file_req in file_recs.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
                    content.push_str(&format!("    lines {}-{} score {:.1}%\n", file_req.line1, file_req.line2, file_req.usefulness));
                }
                used_files.insert(rec.file_name.clone());
            }
        }

        let mut results = vec_context_file_to_context_tools(vector_of_context_file.clone());
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));
        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
