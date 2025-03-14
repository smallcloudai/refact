use crate::at_commands::at_commands::{vec_context_file_to_context_tools, AtCommand, AtCommandsContext, AtParam};
use crate::caps::{get_api_key, ModelType};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tracing::info;
use crate::nicer_logs::last_n_chars;

use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, ContextFile};
use crate::vecdb;
use crate::vecdb::vdb_structs::VecdbSearch;


pub fn text_on_clip(query: &String, from_tool_call: bool) -> String {
    if !from_tool_call {
        return query.clone();
    }
    return format!("performed vecdb search, results below");
}


pub struct AtSearch {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtSearch {
    pub fn new() -> Self {
        AtSearch {
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
        let same_file_again =  vector_of_context_file.iter().map(|x|&x.file_name).filter(|x|**x == file_name).count();
        let same_file_discount = 1. / (same_file_again as f32 * 0.1 + 1.);
        usefulness *= same_file_discount;
        info!("results {} usefulness {:.2} after same-file discount {:.2}",
            last_n_chars(&file_name, 30),
            usefulness,
            same_file_discount,
        );
        vector_of_context_file.push(ContextFile {
            file_name,
            file_content: "".to_string(),
            line1: r.start_line as usize + 1,
            line2: r.end_line as usize + 1,
            symbols: vec![],
            gradient_type: 4,
            usefulness,
        });
    }
    vector_of_context_file
}

pub async fn execute_at_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    query: &String,
    vecdb_scope_filter_mb: Option<String>,
) -> Result<Vec<ContextFile>, String> {
    let (gcx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.top_n)
    };

    let caps_opt = gcx.read().await.caps.clone();
    let provider_name = if let Some(caps) = caps_opt {
        let caps_locked = caps.read().unwrap();
        caps_locked.embedding_provider.clone()
    } else {
        "".to_string()
    };
    let api_key = get_api_key(ModelType::Embedding, gcx.clone(), &provider_name).await;
    if let Err(err) = api_key {
        return Err(err);
    }
    let api_key = api_key.unwrap();

    let vec_db = gcx.read().await.vec_db.clone();
    let r = match *vec_db.lock().await {
        Some(ref db) => {
            let top_n_twice_as_big = top_n * 2;  // top_n will be cut at postprocessing stage, and we really care about top_n files, not pieces
            // TODO: this code sucks, release lock, don't hold anything during the search
            let search_result = db.vecdb_search(query.clone(), top_n_twice_as_big, vecdb_scope_filter_mb, &api_key).await?;
            let results = search_result.results.clone();
            return Ok(results2message(&results));
        }
        None => Err("VecDB is not active. Possible reasons: VecDB is turned off in settings, or perhaps a vectorization model is not available.".to_string())
    };
    r
}

#[async_trait]
impl AtCommand for AtSearch {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        _cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let args1 = args.iter().map(|x|x.clone()).collect::<Vec<_>>();
        info!("execute @search {:?}", args1.iter().map(|x|x.text.clone()).collect::<Vec<_>>());

        let query = args.iter().map(|x|x.text.clone()).collect::<Vec<_>>().join(" ");
        if query.trim().is_empty() {
            if ccx.lock().await.is_preview {
                return Ok((vec![], "".to_string()));
            }
            return Err("Cannot execute search: query is empty.".to_string());
        }

        let vector_of_context_file = execute_at_search(ccx.clone(), &query, None).await?;
        let text = text_on_clip(&query, false);
        Ok((vec_context_file_to_context_tools(vector_of_context_file), text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
