use std::sync::Arc;
use async_trait::async_trait;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use tokio::sync::Mutex as AMutex;
use tracing::info;
use uuid::Uuid;

use crate::vecdb;
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::vecdb::vdb_structs::VecdbSearch;


pub fn text_on_clip(query: &String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query.clone();
    }
    return format!("performed vecdb search, results below");
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

fn results2message(results: &Vec<vecdb::vdb_structs::VecdbRecord>) -> Vec<ContextFile> {
    let mut vector_of_context_file: Vec<ContextFile> = vec![];
    for r in results {
        let file_name = r.file_path.to_str().unwrap().to_string();
        let mut usefulness = r.usefulness;
        // diversifying results
        let chunk_n =  vector_of_context_file.iter().map(|x|&x.file_name).filter(|x|**x == file_name).count();
        usefulness *= 1. / (chunk_n as f32 * 0.1 + 1.);
        // info!("file_name {}; usefulness {}", file_name, usefulness);

        vector_of_context_file.push(ContextFile {
            file_name,
            file_content: r.window_text.clone(),
            line1: r.start_line as usize + 1,
            line2: r.end_line as usize + 1,
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness,
            is_body_important: false
        });
    }
    vector_of_context_file
}

pub async fn execute_at_workspace(ccx: &mut AtCommandsContext, query: &String, vecdb_scope_filter_mb: Option<String>) -> Result<Vec<ContextFile>, String> {
    match *ccx.global_context.read().await.vec_db.lock().await {
        Some(ref db) => {
            let top_n_twice_as_big = ccx.top_n * 2;  // top_n will be cut at postprocessing stage, and we really care about top_n files, not pieces
            let search_result = db.vecdb_search(query.clone(), top_n_twice_as_big, vecdb_scope_filter_mb).await?;
            let results = search_result.results.clone();
            return Ok(results2message(&results));
        }
        None => Err("VecDB is not active. Possible reasons: VecDB is turned off in settings, or perhaps a vectorization model is not available.".to_string())
    }
}

#[async_trait]
impl AtCommand for AtWorkspace {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> { &self.params }
    async fn execute(&self, ccx: &mut AtCommandsContext, _cmd: &mut AtCommandMember, args: &mut Vec<AtCommandMember>) -> Result<(Vec<ContextEnum>, String), String> {
        let args1 = args.iter().map(|x|x.clone()).collect::<Vec<_>>();
        info!("execute @workspace {:?}", args1);
        let query = args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");

        let vector_of_context_file = execute_at_workspace(ccx, &query, None).await?;
        let text = text_on_clip(&query, false);
        Ok((vec_context_file_to_context_tools(vector_of_context_file), text))
    }
    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
