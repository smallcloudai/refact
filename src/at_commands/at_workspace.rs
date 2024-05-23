use std::sync::Arc;
use async_trait::async_trait;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_into_tools};
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::call_validation::{ContextFile, ContextTool};
use crate::vecdb::structs::{Record, VecdbSearch};


fn text_on_clip(query: String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query;
    }
    return format!("performed vecdb search for query: {}", query);
}


pub struct AtWorkspace {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtWorkspace {
    pub fn new() -> Self {
        AtWorkspace {
            name: "@workspace".to_string(),
            params: vec![],
        }
    }
}

fn results2message(results: &Vec<Record>) -> Vec<ContextTool> {
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
    vec_context_file_into_tools(vector_of_context_file)
}

#[async_trait]
impl AtCommand for AtWorkspace {
    fn name(&self) -> &String {
        &self.name
    }

    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>>
    {
        &self.params
    }
    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext, from_tool_call: bool) -> Result<(Vec<ContextTool>, String), String> {
        match *context.global_context.read().await.vec_db.lock().await {
            Some(ref db) => {
                let mut db_query = args.join(" ");
                if db_query.is_empty() {
                    db_query = query.clone();
                }
                let search_result = db.vecdb_search(db_query, top_n).await?;
                let results = search_result.results.clone();
                let text = text_on_clip(args.join(" "), from_tool_call);
                Ok((results2message(&results), text))
            }
            None => Err("vecdb is not available".to_string())
        }
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
