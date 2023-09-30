use ropey::Rope;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock as ARwLock;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};
use tracing::info;

use crate::telemetry_snippets;
use crate::global_context;


// const NAME: &str = "llm-ls";
const VERSION: &str = env!("CARGO_PKG_VERSION");


// #[derive(Serialize)]
// struct APIRequest {
//     inputs: String,
//     parameters: APIParams,
// }

// #[derive(Debug, Deserialize)]
// struct Generation {
//     pub generated_text: String,
// }

#[derive(Debug, Deserialize)]
struct APIError {
    error: String,
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// enum APIResponse {
//     Generation(Generation),
//     Generations(Vec<Generation>),
//     Error(APIError),
// }

#[derive(Debug)]
pub struct Document {
    #[allow(dead_code)]
    pub language_id: String,
    pub text: Rope,
}

impl Document {
    fn new(language_id: String, text: Rope) -> Self {
        Self { language_id, text }
    }
}

#[derive(Debug)]
pub struct Backend {
    pub gcx: Arc<ARwLock<global_context::GlobalContext>>,
    pub client: tower_lsp::Client,
    pub document_map: Arc<ARwLock<HashMap<String, Document>>>,
    pub workspace_folders: Arc<ARwLock<Option<Vec<WorkspaceFolder>>>>,
}


// Maybe support llm-vscode nonstandard call?
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct RequestParams {
//     pub max_new_tokens: u32,
//     pub temperature: f32,
//     pub do_sample: bool,
//     pub top_p: f32,
//     pub stop_tokens: Option<Vec<String>>,
// }
// #[derive(Debug, Deserialize, Serialize)]
// pub struct CompletionParams1 {
//     #[serde(flatten)]
//     pub text_document_position: TextDocumentPositionParams,
//     pub request_params: RequestParams,
//     #[serde(default)]
//     #[serde(deserialize_with = "parse_ide")]
//     pub ide: Ide,
//     // fim: FimParams,
//     pub api_token: Option<String>,
//     pub model: String,
//     pub tokens_to_clear: Vec<String>,
//     // tokenizer_config: Option<TokenizerConfig>,
//     pub context_window: usize,
//     pub tls_skip_verify_insecure: bool,
// }

impl Backend {
    // pub async fn get_completions(&self, params: CompletionParams1) -> Result<Vec<Completion>> {
    //     Ok(vec![Completion { generated_text: "hello".to_owned() }])
    // }
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        *self.workspace_folders.write().await = params.workspace_folders;
        info!("LSP client_info {:?}", params.client_info);
        info!("LSP workspace_folders {:?}", self.workspace_folders);

        let completion_options: CompletionOptions;
        completion_options = CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".(".to_owned()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions { work_done_progress: Some(false) },
            completion_item: None,
        };
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "llm-ls".to_owned(),
                version: Some(VERSION.to_owned()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(completion_options),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rust LSP received initialized()")
            .await;
        let _ = info!("rust LSP received initialized()");
    }

    // TODO:
    // textDocument/didClose

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{llm-ls} file opened")
            .await;
        let rope = ropey::Rope::from_str(&params.text_document.text);
        let uri = params.text_document.uri.to_string();
        *self
            .document_map
            .write()
            .await
            .entry(uri.clone())
            .or_insert(Document::new("unknown".to_owned(), Rope::new())) =
            Document::new(params.text_document.language_id, rope);
        info!("{uri} opened");
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let t0 = Instant::now();
        self.client
            .log_message(MessageType::INFO, "{llm-ls} file changed")
            .await;
        let rope = ropey::Rope::from_str(&params.content_changes[0].text);
        let uri = params.text_document.uri.to_string();
        let mut document_map = self.document_map.write().await;
        let doc = document_map
            .entry(uri.clone())
            .or_insert(Document::new("unknown".to_owned(), Rope::new()));
        doc.text = rope;
        info!("{} changed, save time: {:?}", uri, t0.elapsed());
        let t1 = Instant::now();
        telemetry_snippets::sources_changed(
            self.gcx.clone(),
            &uri,
            &params.content_changes[0].text
        ).await;
        info!("{} changed, telemetry time: {:?}", uri, t1.elapsed());
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{llm-ls} file saved")
            .await;
        let uri = params.text_document.uri.to_string();
        info!("{uri} saved");
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{llm-ls} file closed")
            .await;
        let uri = params.text_document.uri.to_string();
        info!("{uri} closed");
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = info!("shutdown");
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("asked for completion");
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string())
        ])))
    }
}

pub fn build_lsp_service(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> (LspService::<Backend>, ClientSocket) {
    let (lsp_service, socket) = LspService::build(|client| Backend {
        gcx,
        client,
        document_map: Arc::new(ARwLock::new(HashMap::new())),
        workspace_folders: Arc::new(ARwLock::new(None)),
    })
    // .custom_method("llm-ls/getCompletions", Backend::get_completions)
    .finish();
    (lsp_service, socket)
}
