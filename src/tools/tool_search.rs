use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommandsContext};
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::at_commands::at_search::execute_at_search;
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs};
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ContextFile};


pub struct ToolSearch;

async fn execute_att_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &String,
    scope: &String,
) -> Result<Vec<ContextFile>, String> {
    fn is_scope_a_file(scope: &String, candidates_file: &Vec<String>) -> bool {
        if !candidates_file.is_empty() {
            return true
        }
        PathBuf::from(scope).extension().is_some()
    }
    fn is_scope_a_dir(scope: &String, candidates_dir: &Vec<String>) -> bool {
        if !candidates_dir.is_empty() {
            return true
        }
        let path = PathBuf::from(scope);
        match fs::metadata(&path) {
            Ok(metadata) => metadata.is_dir(),
            Err(_) => false,
        }
    }
    let gcx = ccx.lock().await.global_context.clone();
    let candidates_file = file_repair_candidates(gcx.clone(), scope, 10, false).await;
    let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), scope, false, 10).await;

    return match scope.as_str() {
        "workspace" => {
            Ok(execute_at_search(ccx.clone(), &query, None).await?)
        }
        _ if is_scope_a_file(scope, &candidates_file) => {
            let file_path = return_one_candidate_or_a_good_error(
                gcx.clone(),
                scope,
                &candidates_file,
                &get_project_dirs(gcx.clone()
                ).await, false).await?;
            let filter = Some(format!("(scope = \"{}\")", file_path));
            info!("att-search: filter: {:?}", filter);
            Ok(execute_at_search(ccx.clone(), &query, filter).await?)
        }
        _ if is_scope_a_dir(scope, &candidates_dir) => {
            let dir = return_one_candidate_or_a_good_error(
                gcx.clone(),
                scope,
                &candidates_dir,
                &get_project_dirs(gcx.clone()
                ).await, true).await?;
            let filter = format!("(scope LIKE '{}%')", dir);
            info!("att-search: filter: {:?}", filter);
            Ok(execute_at_search(ccx.clone(), &query, Some(filter)).await?)
        }
        _ => Err(format!("scope {} is not supported", scope))
    };
}

#[async_trait]
impl Tool for ToolSearch {
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
            return Err("search has given no results. Adjust a query or try a different scope".to_string());
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
            content,
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
