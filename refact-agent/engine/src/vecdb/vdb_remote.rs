use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde_json::json;

use crate::vecdb::vdb_structs::{SearchResult, VecdbSearch};


#[derive(Debug)]
pub struct VecDbRemote {}

#[async_trait]
impl VecdbSearch for VecDbRemote {
    async fn vecdb_search(
        &self,
        query: String,
        top_n: usize,
        _vecdb_scope_filter_mb: Option<String>,
    ) -> Result<SearchResult, String> {
        // NOTE: if you're going to use https make sure that you set insecure flag from cmdline
        let url = "http://127.0.0.1:8008/v1/vdb-search".to_string();
        let mut headers = HeaderMap::new();
        // headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap());
        headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
        let body = json!({
            "text": query,
            "top_n": top_n
        });
        let res = reqwest::Client::new()
            .post(&url)
            .headers(headers)
            .body(body.to_string())
            .send()
            .await.map_err(|e| format!("Vecdb search HTTP error (1): {}", e))?;

        let body = res.text().await.map_err(|e| format!("Vecdb search HTTP error (2): {}", e))?;
        // info!("Vecdb search result: {:?}", &body);
        let result: Vec<SearchResult> = serde_json::from_str(&body).map_err(|e| {
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
