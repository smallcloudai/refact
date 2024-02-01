use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;
use crate::at_commands::structs::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamFilePath;
use tokio::sync::Mutex as AMutex;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::vecdb::vecdb::FileSearchResult;

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
        match *context.global_context.read().await.vec_db.lock().await {
            Some(ref db) => {
                if !can_execute {
                    return Err("incorrect arguments".to_string());
                }
                let file_path = match args.get(0) {
                    Some(x) => x,
                    None => return Err("no file path".to_string()),
                };
                let path_and_text: FileSearchResult = db.get_file_orig_text(file_path.clone()).await;
                let mut vector_of_context_file: Vec<ContextFile> = vec![];
                vector_of_context_file.push(ContextFile {
                    file_name: path_and_text.file_path,
                    file_content: path_and_text.file_text.clone(),
                    line1: 0,
                    line2: path_and_text.file_text.lines().count() as i32,
                });
                Ok(ChatMessage {
                    role: "context_file".to_string(),
                    content: json!(vector_of_context_file).to_string(),
                })
            }
            None => Err("vecdb is not available".to_string())
        }
    }
}
