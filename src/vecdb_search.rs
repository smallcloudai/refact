use crate::call_validation::{ChatMessage, ChatPost};
// use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use serde_json::json;

use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VecdbResultRec {
    pub file_name: String,
    pub text: String,
    pub score: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VecdbResult {
    pub results: Vec<VecdbResultRec>,
}

pub async fn embed_vecdb_results(
    vecdb_search: Arc<AMutex<Box<dyn VecdbSearch + Send>>>,
    post: &mut ChatPost,
    limit_examples_cnt: usize,
) {
    let my_vdb = vecdb_search.clone();
    let latest_msg_cont = &post.messages.last().unwrap().content;
    let mut vecdb_locked = my_vdb.lock().await;
    let vdb_resp = vecdb_locked.search(&latest_msg_cont).await;
    let vdb_cont = vecdb_resp_to_prompt(&vdb_resp, limit_examples_cnt);
    if vdb_cont.len() > 0 {
        post.messages = [
            &post.messages[..post.messages.len() -1],
            &[ChatMessage {
                role: "user".to_string(),
                content: vdb_cont,
            }],
            &post.messages[post.messages.len() -1..],
        ].concat();
    }
}


fn vecdb_resp_to_prompt(
    resp: &Result<VecdbResult, String>,
    limit_examples_cnt: usize,
) -> String {
    let mut cont = "".to_string();
    match resp {
        Ok(resp) => {
            cont.push_str("CONTEXT:\n");
            for i in 0..limit_examples_cnt {
                if i >= resp.results.len() {
                    break;
                }
                cont.push_str("FILENAME:\n");
                cont.push_str(resp.results[i].file_name.clone().as_str());
                cont.push_str("\nTEXT:");
                cont.push_str(resp.results[i].text.clone().as_str());
                cont.push_str("\n");
            }
            cont.push_str("\nRefer to the context to answer my next question.\n");
            cont
        }
        Err(e) => {
            format!("Vecdb error: {}", e);
            cont
        }
    }
}

#[async_trait]
pub trait VecdbSearch: Send {
    async fn search(
        &mut self,
        query: &str,
    ) -> Result<VecdbResult, String>;
}

#[derive(Debug, Clone)]
pub struct VecdbSearchTest {
}

impl VecdbSearchTest {
    pub fn new() -> Self {
        VecdbSearchTest {
        }
    }
}

// unsafe impl Send for VecdbSearchTest {}

#[async_trait]
impl VecdbSearch for VecdbSearchTest {
    async fn search(
        &mut self,
        query: &str,
    ) -> Result<VecdbResult, String> {
        let url = "http://127.0.0.1:8008/v1/vdb-search".to_string();
        let mut headers = HeaderMap::new();
        // headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap());
        headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
        let body = json!({
            "texts": [query],
            "account": "XXX",
            "top_k": 3,
        });
        let res = reqwest::Client::new()
            .post(&url)
            .headers(headers)
            .body(body.to_string())
            .send()
            .await.map_err(|e| format!("Vecdb search HTTP error (1): {}", e))?;

        let body = res.text().await.map_err(|e| format!("Vecdb search HTTP error (2): {}", e))?;
        // info!("Vecdb search result: {:?}", &body);
        let result: Vec<VecdbResult> = serde_json::from_str(&body).map_err(|e| {
            format!("vecdb JSON problem: {}", e)
        })?;
        if result.len() == 0 {
            return Err("Vecdb search result is empty".to_string());
        }
        let result0 = result[0].clone();
        // info!("Vecdb search result: {:?}", &result0);
        Ok(result0)
    }
}
