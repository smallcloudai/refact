use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};

use crate::call_validation::{ChatMessage, ContextFile};
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamFilePath;
use crate::files_in_workspace::pathbuf_to_url;
use crate::global_context::GlobalContext;


pub struct AtFile {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtFile {
    pub fn new() -> Self {
        AtFile {
            name: "@file".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtFile {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn are_args_valid(&self, args: &Vec<String>, context: &AtCommandsContext) -> Vec<bool> {
        let mut results = Vec::new();
        for (arg, param) in args.iter().zip(self.params.iter()) {
            let param = param.lock().await;
            results.push(param.is_value_valid(arg, context).await);
        }
        results
    }

    async fn can_execute(&self, args: &Vec<String>, context: &AtCommandsContext) -> bool {
        if self.are_args_valid(args, context).await.iter().any(|&x| x == false) || args.len() != self.params.len() {
            return false;
        }
        return true;
    }

    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }
        let file_path = match args.get(0) {
            Some(x) => x,
            None => return Err("no file path".to_string()),
        };

        let file_text = get_file_text(context.global_context.clone(), file_path).await?;

        let mut vector_of_context_file: Vec<ContextFile> = vec![];
        vector_of_context_file.push(ContextFile {
            file_name: file_path.clone(),
            file_content: file_text.clone(),
            line1: 0,
            line2: file_text.lines().count() as i32,
            usefullness: 100.0,
        });
        Ok(ChatMessage {
            role: "context_file".to_string(),
            content: json!(vector_of_context_file).to_string(),
        })
    }
}

async fn get_file_text(global_context: Arc<ARwLock<GlobalContext>>, file_path: &String) -> Result<String, String> {
    let cx = global_context.read().await;

    // if you write pathbuf_to_url(&PathBuf::from(file_path)) without unwrapping it gives: future cannot be sent between threads safe
    let url_mb = pathbuf_to_url(&PathBuf::from(file_path)).map(|x|Some(x)).unwrap_or(None);
    if let Some(url) = url_mb {
        let document_mb = cx.documents_state.document_map.read().await.get(&url).cloned();
        if document_mb.is_some() {
            return Ok(document_mb.unwrap().text.to_string());
        }
    }

    return match *cx.vec_db.lock().await {
        Some(ref db) => Ok(db.get_file_orig_text(file_path.clone()).await.file_text),
        None => Err("vecdb is not available && no file in memory was found".to_string())
    };
}
