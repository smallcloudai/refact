use std::sync::Arc;
use async_trait::async_trait;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::vecdb::structs::{Record, VecdbSearch};


pub fn text_on_clip(query: &String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query.clone();
    }
    return format!("performed vecdb search for query: {}", query);
}


pub struct AtWorkspace {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtWorkspace {
    pub fn new() -> Self {
        AtWorkspace {
            params: vec![],
        }
    }
}

fn results2message(results: &Vec<Record>) -> Vec<ContextFile> {
    let mut vector_of_context_file: Vec<ContextFile> = vec![];
    for i in 0..results.len() {
        let r = &results[i];
        vector_of_context_file.push(ContextFile {
            file_name: r.file_path.to_str().unwrap().to_string(),
            file_content: r.window_text.clone(),
            line1: r.start_line as usize + 1,
            line2: r.end_line as usize + 1,
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness: r.usefulness,
            is_body_important: false
        });
    }
    vector_of_context_file
}

pub async fn execute_at_workspace(ccx: &mut AtCommandsContext, query: &String, args: &Vec<String>) -> Result<Vec<ContextFile>, String> {
    match *ccx.global_context.read().await.vec_db.lock().await {
        Some(ref db) => {
            let mut db_query = args.join(" ");
            if db_query.is_empty() {
                db_query = query.clone();
            }
            let search_result = db.vecdb_search(db_query, ccx.top_n).await?;
            let results = search_result.results.clone();
            return Ok(results2message(&results));
        }
        None => Err("vecdb is not available".to_string())
    }
}

#[async_trait]
impl AtCommand for AtWorkspace {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>
    {
        &self.params
    }
    async fn execute(&self, ccx: &mut AtCommandsContext, query: &String, args: &Vec<String>) -> Result<(Vec<ContextEnum>, String), String> {
        let vector_of_context_file = execute_at_workspace(ccx, query, args).await?;
        let text = text_on_clip(query, false);
        Ok((vec_context_file_to_context_tools(vector_of_context_file), text))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
