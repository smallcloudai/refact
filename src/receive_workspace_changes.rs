use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Instant;
use ropey::Rope;

use tokio::sync::RwLock as ARwLock;
use tracing::{info};

use crate::global_context;
use crate::telemetry;



#[derive(Debug)]
pub struct Document {
    #[allow(dead_code)]
    pub language_id: String,
    pub text: Rope,
}

impl Document {
    pub fn new(language_id: String, text: Rope) -> Self {
        Self { language_id, text }
    }
}


pub async fn on_did_open(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    uri: &String,
    text: &String,
    language_id: &String,
) {
    let t0 = Instant::now();
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.lsp_backend_document_state.document_map;
    let rope = ropey::Rope::from_str(&text);
    let mut document_map_locked = document_map.write().await;
    *document_map_locked
        .entry(uri.clone())
        .or_insert(Document::new("unknown".to_owned(), Rope::new())) =
        Document::new(language_id.clone(), rope);
    info!("{} opened, save time: {:?}", uri, t0.elapsed());
}

pub async fn on_did_change(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    uri: &String,
    text: &String,
) {
    let t0 = Instant::now();

    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.lsp_backend_document_state.document_map;
    let rope = ropey::Rope::from_str(&text);
    let mut document_map_locked = document_map.write().await;
    let doc = document_map_locked
        .entry(uri.clone())
        .or_insert(Document::new("unknown".to_owned(), Rope::new()));
    doc.text = rope;

    info!("{} changed, save time: {:?}", uri, t0.elapsed());
    let t1 = Instant::now();
    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        uri,
        text,
    ).await;
    info!("{} changed, telemetry time: {:?}", uri, t1.elapsed());
}
