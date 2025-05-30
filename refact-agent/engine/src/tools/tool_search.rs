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
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};


pub struct ToolSearch {
    pub config_path: String,
}

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

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "search_semantic".to_string(),
            display_name: "Search".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Find semantically similar pieces of code or text using vector database (semantic search)".to_string(),
            parameters: vec![
                ToolParam {
                    name: "queries".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of queries. Each query can be a single line, paragraph or code sample to search for semantically similar content.".to_string(),
                },
                ToolParam {
                    name: "scope".to_string(),
                    param_type: "string".to_string(),
                    description: "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file.".to_string(),
                }
            ],
            parameters_required: vec!["queries".to_string(), "scope".to_string()],
        }
    }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let query_str = match args.get("queries") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `queries` is not a string: {:?}", v)),
            None => return Err("Missing argument `queries` in the search_semantic() call.".to_string())
        };
        let scope = match args.get("scope") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `scope` is not a string: {:?}", v)),
            None => return Err("Missing argument `scope` in the search_semantic() call.".to_string())
        };

        let queries: Vec<String> = query_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if queries.is_empty() {
            return Err("No valid queries provided".to_string());
        }

        let mut all_context_files = Vec::new();
        let mut all_content = String::new();

        for (i, query) in queries.iter().enumerate() {
            if i > 0 {
                all_content.push_str("\n\n");
            }
            
            all_content.push_str(&format!("Results for query: \"{}\"\n", query));
            
            let vector_of_context_file = execute_att_search(ccx.clone(), query, &scope).await?;
            info!("att-search: vector_of_context_file={:?}", vector_of_context_file);

            if vector_of_context_file.is_empty() {
                all_content.push_str("No results found for this query.\n");
                continue;
            }

            all_content.push_str("Records found:\n\n");
            let mut file_results_to_reqs: HashMap<String, Vec<&ContextFile>> = HashMap::new();
            vector_of_context_file.iter().for_each(|rec| {
                file_results_to_reqs.entry(rec.file_name.clone()).or_insert(vec![]).push(rec)
            });
            
            let mut used_files: HashSet<String> = HashSet::new();
            for rec in vector_of_context_file.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
                if !used_files.contains(&rec.file_name) {
                    all_content.push_str(&format!("{}:\n", rec.file_name.clone()));
                    let file_recs = file_results_to_reqs.get(&rec.file_name).unwrap();
                    for file_req in file_recs.iter().sorted_by(|rec1, rec2| rec2.usefulness.total_cmp(&rec1.usefulness)) {
                        all_content.push_str(&format!("    lines {}-{} score {:.1}%\n", file_req.line1, file_req.line2, file_req.usefulness));
                    }
                    used_files.insert(rec.file_name.clone());
                }
            }

            all_context_files.extend(vector_of_context_file);
        }

        if all_context_files.is_empty() {
            return Err("All searches produced no results, adjust the queries or try a different scope.".to_string());
        }

        let mut results = vec_context_file_to_context_tools(all_context_files);
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(all_content),
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
