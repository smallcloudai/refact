use std::sync::Arc;
use async_trait::async_trait;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::call_validation::ContextFile;
use crate::vecdb::structs::{Record, VecdbSearch};


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
        });
    }
    vector_of_context_file
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
    async fn execute(&self, query: &String, args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<Vec<ContextFile>, String> {
        match *context.global_context.read().await.vec_db.lock().await {
            Some(ref db) => {
                let mut db_query = args.join(" ");
                if db_query.is_empty() {
                    db_query = query.clone();
                }
                let search_result = db.vecdb_search(db_query, top_n).await?;
                let results = search_result.results.clone();
                Ok(results2message(&results))
            }
            None => Err("vecdb is not available".to_string())
        }
    }
}
