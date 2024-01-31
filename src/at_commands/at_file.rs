use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::at_commands::structs::{AtCommand, AtCommandsContext, AtParam, AtParamKind};
use crate::at_commands::at_params::AtParamFilePath;
use tokio::sync::Mutex as AMutex;
use crate::at_commands::utils::compose_context_file_msg_from_result;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::vecdb::vecdb::FileSearchResult;

#[derive(Debug)]
pub struct AtFile {
    pub name: String,
    pub params: Vec<Arc<AMutex<AtParamKind>>>,
}

impl AtFile {
    pub fn new() -> Self {
        AtFile {
            name: "@file".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamKind::AtParamFilePath(AtParamFilePath::new())))
            ],
        }
    }
}

fn search2messages(result: &FileSearchResult) -> Vec<ChatMessage> {
    // TODO: change to context_file, encode json including line1 line2
    vec![ChatMessage {
        role: "user".to_string(),
        content: format!("FILENAME:\n{}\nTEXT:\n{}\n", result.file_path, result.file_text)
    }]
}

fn search2json(result: &FileSearchResult) -> Value {
    let cf = ContextFile {
        file_name: result.file_path.clone().rsplit("/").next().unwrap_or(&result.file_path).to_string(),
        file_content: result.file_text.clone(),
        line1: 0,
        line2: result.file_text.lines().count() as i32,
    };
    compose_context_file_msg_from_result(&serde_json::to_value(&vec![cf]).unwrap_or(json!(null)))
}

#[async_trait]
impl AtCommand for AtFile {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<AtParamKind>>> {
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

    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ChatMessage>, Value), String> {
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
                let search_result = db.get_file_orig_text(file_path.clone()).await;
                Ok((
                    search2messages(&search_result),
                    search2json(&search_result)
                ))
            }
            None => Err("vecdb is not available".to_string())
        }
    }
}
