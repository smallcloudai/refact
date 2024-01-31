use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::at_commands::structs::{AtCommand, AtCommandsContext, AtParam, AtParamKind};
use tokio::sync::Mutex as AMutex;
use crate::at_commands::utils::compose_context_file_msg_from_result;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::vecdb::structs::{Record, VecdbSearch};


#[derive(Debug)]
pub struct AtWorkspace {
    pub name: String,
    pub params: Vec<Arc<AMutex<AtParamKind>>>,
}

impl AtWorkspace {
    pub fn new() -> Self {
        AtWorkspace {
            name: "@workspace".to_string(),
            params: vec![],
        }
    }
}

fn record2chat_message(record: &Record) -> ChatMessage {
    ChatMessage {
        role: "context_file".to_string(),
        content: json!(ContextFile {
            file_name: record.file_path.to_str().unwrap().to_string(), //.rsplit('/').next().unwrap_or(&record.file_path.to_str().unwrap()).to_string(),
            file_content: record.window_text.clone(),
            line1: record.start_line as i32,
            line2: record.end_line as i32,
        }).to_string()
    }
}

fn search2messages(results: &Vec<Record>) -> Vec<ChatMessage> {
    let mut messages = vec![];
    for r in results {
        messages.push(record2chat_message(r));
    }
    messages
}

pub fn search2json(
    results: &Vec<Record>
) -> Value {
    let context_files: Vec<ChatMessage> = results
        .iter()
        .map(|x| { record2chat_message(x) }).collect();
    compose_context_file_msg_from_result(&serde_json::to_value(&context_files).unwrap_or(json!(null)))
}

#[async_trait]
impl AtCommand for AtWorkspace {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<AtParamKind>>>
    {
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

    async fn execute(&self, query: &String, _args: &Vec<String>, top_n: usize, context: &AtCommandsContext) -> Result<(Vec<ChatMessage>, Value), String> {
        match *context.global_context.read().await.vec_db.lock().await {
            Some(ref db) => {
                let search_result = db.search(query.clone(), top_n).await?;
                let mut results = search_result.results.clone();
                results.dedup_by(|a, b| a.file_path == b.file_path && a.window_text == b.window_text);

                Ok((
                    search2messages(&results),
                    search2json(&results)
                ))
            }
            None => Err("vecdb is not available".to_string())
        }
    }
}
