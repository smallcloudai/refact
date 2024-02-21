use std::hash::Hash;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use ropey::Rope;
use tokio::fs::read_to_string;

use tokio::sync::RwLock as ARwLock;
use tracing::{debug, info};
use tracing_subscriber::fmt::format;
use url::Url;

use crate::global_context;
use crate::telemetry;
use crate::vecdb::file_filter;
use crate::vecdb::file_filter::is_valid_file;


#[derive(Debug, Eq, Hash, PartialEq, Clone)]
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

#[derive(Debug, Clone, Eq)]
pub struct DocumentInfo {
    pub uri: Url,
    pub document: Option<Document>
}

impl PartialEq<Self> for DocumentInfo {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
    }
}
impl Hash for DocumentInfo {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.uri.hash(state);
    }
}

impl DocumentInfo {
    pub fn from_pathbuf(path: &PathBuf) -> Result<Self, String> {
        match pathbuf_to_url(path) {
            Ok(uri) => Ok(Self { uri, document: None }),
            Err(_) => Err("Failed to convert path to URL".to_owned())
        }
    }

    pub fn from_pathbuf_and_text(path: &PathBuf, text: &String) -> Result<Self, String> {
        match pathbuf_to_url(path) {
            Ok(uri) => Ok(Self {
                uri,
                document: Some(Document {
                    language_id: "unknown".to_string(),
                    text: Rope::from_str(&text),
                }),
            }),
            Err(_) => Err("Failed to convert path to URL".to_owned())
        }
    }

    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(self.uri.path())
    }

    pub async fn read_file(&self) -> io::Result<String> {
        match &self.document {
            Some(doc) => Ok(doc.text.to_string()),
            None => {
                read_to_string(self.get_path()).await
            }
        }
    }
}

pub fn pathbuf_to_url(path: &PathBuf) -> Result<Url, Box<dyn std::error::Error>> {
    let absolute_path = if path.is_absolute() {
        path.clone()
    } else {
        let path = std::env::current_dir()?.join(path);
        path
    };
    let url = Url::from_file_path(absolute_path).map_err(|_| "Failed to convert path to URL")?;
    Ok(url)
}


pub async fn enqueue_all_files_from_workspace_folders(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    let folders: Vec<PathBuf> = {
        let cx_locked = gcx.read().await;
        let x = cx_locked.documents_state.workspace_folders.lock().unwrap().clone();
        x
    };
    info!("enqueue_all_files_from_workspace_folders started files search with {} folders", folders.len());
    let docs = file_filter::retrieve_files_by_proj_folders(folders).await;
    info!("enqueue_all_files_from_workspace_folders found {} files", docs.len());
    let (ast_module, vecdb_module) = {
        let cx_locked = gcx.read().await;
        (cx_locked.ast_module.clone(), cx_locked.vec_db.clone())
    };
    match *ast_module.lock().await {
        Some(ref mut ast) => ast.ast_indexer_enqueue_files(&docs, true).await,
        None => {
            info!("ast_module is None");
        },
    };
    match *vecdb_module.lock().await {
        Some(ref mut db) => db.vectorizer_enqueue_files(&docs, false).await,
        None => {},
    };
    docs.len() as i32
}

pub async fn on_workspaces_init(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> i32 {
    // TODO: this will not work when files change. Need a real file watcher.
    enqueue_all_files_from_workspace_folders(gcx.clone()).await
}

pub async fn on_did_open(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    file_url: &Url,
    text: &String,
    language_id: &String,
) {
    let gcx_locked = gcx.read().await;
    let document_map = &gcx_locked.documents_state.document_map;
    let mut document_map_locked = document_map.write().await;
    let doc = Document::new(language_id.clone(), Rope::from_str(&text));
    let doc_info = DocumentInfo { uri: file_url.clone(), document: Some(doc.clone()) };
    document_map_locked.insert(file_url.to_string(), doc);
    let path_str = format!("{:?}", doc_info.get_path());
    let last_30_chars: String = crate::nicer_logs::last_n_chars(&path_str, 30);
    info!("opened {}", last_30_chars);
}


pub async fn on_did_change(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    file_url: &Url,
    text: &String,
) {
    let t0 = Instant::now();
    let doc_info = {
        let gcx_locked = gcx.read().await;
        let document_map = &gcx_locked.documents_state.document_map;
        let mut document_map_locked = document_map.write().await;
        let doc = document_map_locked.entry(file_url.to_string())
            .or_insert(Document::new("unknown".to_owned(), Rope::new()));
        doc.text = Rope::from_str(&text);
        DocumentInfo { uri: file_url.clone(), document: Some(doc.clone()) }
    };
    if is_valid_file(&doc_info.get_path()) {
        {
            let vecdb_bind = gcx.read().await.vec_db.clone();
            match *vecdb_bind.lock().await {
                Some(ref mut db) => db.vectorizer_enqueue_files(&vec![doc_info.clone()], false).await,
                None => {}
            };
        }
        {
            let ast_bind = gcx.read().await.ast_module.clone();
            match *ast_bind.lock().await {
                Some(ref mut ast) => ast.ast_indexer_enqueue_files(&vec![doc_info.clone()], false).await,
                None => {}
            };
        }
    }

    telemetry::snippets_collection::sources_changed(
        gcx.clone(),
        &doc_info.uri.to_string(),
        text,
    ).await;
    let last_30_chars: String = crate::nicer_logs::last_n_chars(&doc_info.get_path().display().to_string(), 30);
    info!("changed {}, total time {:.3}s", last_30_chars, t0.elapsed().as_secs_f32());
}
