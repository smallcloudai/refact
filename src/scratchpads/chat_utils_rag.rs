use async_trait::async_trait;
use serde_json::json;
use tracing::info;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile};
use crate::vecdb::structs::{SearchResult, VecdbSearch};

pub async fn chat_functions_middleware<T>(
    vecdb: &T,
    post: &mut ChatPost,
    limit_examples_cnt: usize,
    has_vecdb: &mut dyn HasVecdb,
) where T: VecdbSearch {
    let latest_msg_cont = &post.messages.last().unwrap().content;
    if latest_msg_cont.starts_with("@workspace") {
        embed_vecdb_results(vecdb, post, limit_examples_cnt, has_vecdb).await;
    }
}


async fn embed_vecdb_results<T>(
    vecdb: &T,
    post: &mut ChatPost,
    limit_examples_cnt: usize,
    has_vecdb: &mut dyn HasVecdb,
) where T: VecdbSearch {
    let latest_msg_cont = &post.messages.last().unwrap().content;
    let vdb_resp = vecdb.search(latest_msg_cont.clone(), limit_examples_cnt).await;

    has_vecdb.add2messages(
        vdb_resp,
        &mut post.messages,
    ).await;
}

fn vecdb_resp_to_json(
    resp: &Result<SearchResult, String>
) -> serde_json::Result<serde_json::Value> {
    let mut context_files: Vec<ContextFile> = match resp {
        Ok(search_res) => {
            search_res.results.iter().map(|x| ContextFile {
                file_name: x.file_path.to_str().unwrap().to_string(),
                file_content: x.window_text.clone(),
                line1: x.start_line as i32,
                line2: x.end_line as i32,
            }).collect()
        }
        Err(_) => vec![],
    };

    context_files.dedup_by(|a, b| {
        a.file_name == b.file_name && a.file_content == b.file_content
    });

    context_files.iter_mut().for_each(|file| {
        file.file_name = file.file_name
            .rsplit('/')
            .next()
            .unwrap_or(&file.file_name)
            .to_string();
    });

    serde_json::to_value(&context_files)
}

fn vecdb_resp_to_prompt(
    resp_mb: &Result<SearchResult, String>
) -> String {
    let mut cont = "".to_string();

    if resp_mb.is_err() {
        info!("VECDB ERR");
        return cont
    }
    let resp = resp_mb.as_ref().unwrap();
    let mut results = resp.results.clone();
    results.dedup_by(|a, b| a.file_path == b.file_path && a.window_text == b.window_text);

    cont.push_str("CONTEXT:\n");
    for res in results.iter() {
        cont.push_str("FILENAME:\n");
        cont.push_str(res.file_path.clone().to_str().unwrap_or_else( || ""));
        cont.push_str("\nTEXT:");
        cont.push_str(res.window_text.clone().as_str());
        cont.push_str("\n");
    }
    cont.push_str("\nRefer to the context to answer my next question.\n");
    info!("VECDB prompt:\n{}", cont);
    cont
}


pub struct HasVecdbResults {
    pub was_sent: bool,
    pub in_json: serde_json::Value,
}

impl HasVecdbResults {
    pub fn new() -> Self {
        HasVecdbResults {
            was_sent: false,
            in_json: json!(null)
        }
    }
}

#[async_trait]
pub trait HasVecdb: Send {
    async fn add2messages(
        &mut self,
        vdb_result_mb: Result<SearchResult, String>,
        messages: &mut Vec<ChatMessage>,
    );
    fn response_streaming(&mut self) -> Result<serde_json::Value, String>;
}

#[async_trait]
impl HasVecdb for HasVecdbResults {
    async fn add2messages(
        &mut self,
        result_mb: Result<SearchResult, String>,
        messages: &mut Vec<ChatMessage>,
    ) {
        // if messages.len() > 1 {
        //     return;
        // }

        *messages = [
            &messages[..messages.len() -1],
            &[ChatMessage {
                role: "user".to_string(),
                content: vecdb_resp_to_prompt(&result_mb),
            }],
            &messages[messages.len() -1..],
        ].concat();

        self.in_json = vecdb_resp_to_json(&result_mb).unwrap_or_else(|_| json!(null));
    }

    fn response_streaming(&mut self) -> Result<serde_json::Value, String> {
        if self.was_sent == true || self.in_json.is_null() {
            return Ok(json!(null));
        }
        self.was_sent = true;
        return Ok(json!({
            "choices": [{
                "delta": {
                    "content": self.in_json.clone(),
                    "role": "context_file"
                },
                "finish_reason": serde_json::Value::Null,
                "index": 0
            }],
        }));
    }
}
