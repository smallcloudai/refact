use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Instant;
use ropey::Rope;

use tokio::sync::RwLock as ARwLock;
use tracing::info;

use crate::global_context;
use crate::lsp::document::Document;
use crate::telemetry;


pub async fn on_did_open(
    gcx: Arc<ARwLock<global_context::GlobalContext<'_>>>,
    uri: &String,
    text: &String,
    language_id: &String,
) {
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.lsp_backend_document_state.document_map;
    let mut document_map_locked = document_map.write().await;
    match Document::open(language_id, text, uri) {
        Ok(doc) => {
            match document_map_locked.entry(uri.clone()) {
                Entry::Occupied(mut entry) => {
                    entry.insert(doc);
                }
                Entry::Vacant(entry) => {
                    entry.insert(doc);
                }
            }
        }
        Err(_) => {}
    }
    let last_30_chars: String = uri.chars().rev().take(30).collect::<String>().chars().rev().collect();
    info!("opened ...{}", last_30_chars);
}

pub async fn on_did_change(
    gcx: Arc<ARwLock<global_context::GlobalContext<'_>>>,
    uri: &String,
    text: &String,
) {
    let t0 = Instant::now();
    {
        let gcx_locked = gcx.read().await;
        let document_map = &gcx_locked.lsp_backend_document_state.document_map;
        let mut document_map_locked = document_map.write().await;

        let doc = document_map_locked.entry(uri.clone());
        match doc {
            Entry::Occupied(entry) => {
                entry.into_mut().change(text).await.expect("TODO: panic message");
            }
            Entry::Vacant(entry) => {
                match Document::open("python", text, uri) {
                    Ok(d) => {
                        entry.insert(d);
                    }
                    Err(_) => {}
                }
            }
        }
    }
    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        uri,
        text,
    ).await;
    let last_30_chars: String = uri.chars().rev().take(30).collect::<String>().chars().rev().collect();
    info!("changed ...{}, total time {:?}", last_30_chars, t0.elapsed());
}
