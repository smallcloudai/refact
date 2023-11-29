use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Instant;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tower_lsp::{ClientSocket, LanguageServer, LspService};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tracing::{error, info};

use crate::call_validation::{CodeCompletionInputs, CodeCompletionPost, CursorPosition, SamplingParameters};
use crate::global_context;
use crate::global_context::CommandLine;
use crate::http::routers::v1::code_completion::handle_v1_code_completion;
use crate::telemetry;

const VERSION: &str = env!("CARGO_PKG_VERSION");


#[derive(Debug, Deserialize)]
struct APIError {
    error: String,
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}


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

// #[derive(Debug)]  GlobalContext does not implement Debug
pub struct Backend {
    pub gcx: Arc<ARwLock<global_context::GlobalContext>>,
    pub client: tower_lsp::Client,
    pub document_map: Arc<ARwLock<HashMap<String, Document>>>,
    pub workspace_folders: Arc<ARwLock<Option<Vec<WorkspaceFolder>>>>,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestParams {
    pub max_new_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Completion {
    generated_text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionParams1 {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
    pub parameters: RequestParams,
    pub multiline: bool,
    // pub model: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestHeadTailAddedText {
    pub text_a: String,
    pub text_b: String,
    pub orig_grey_text: String,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct TestHeadTailAddedTextRes {
    pub is_valid: bool,
    pub grey_corrected: String,
    pub unchanged_percentage: f64,
}


fn internal_error<E: Display>(err: E) -> Error {
    let err_msg = err.to_string();
    error!(err_msg);
    Error {
        code: tower_lsp::jsonrpc::ErrorCode::InternalError,
        message: err_msg.into(),
        data: None,
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Choice {
    pub index: u32,
    pub code_completion: String,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CompletionRes {
    pub choices: Vec<Choice>,
    pub cached: Option<bool>,
    pub snippet_telemetry_id: u32,
    pub model: String,
    pub created: Option<f32>,
}

impl Backend {
    async fn flat_params_to_code_completion_post(&self, params: &CompletionParams1) -> CodeCompletionPost {
        let document_map = self.document_map.read().await;
        let document = document_map
            .get(params.text_document_position.text_document.uri.as_str())
            .unwrap();
        let txt = &document.text;
        CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from([(String::from(&params.text_document_position.text_document.uri.to_string()),
                                         (&txt).to_string())]),
                cursor: CursorPosition {
                    file: String::from(&params.text_document_position.text_document.uri.to_string()),
                    line: params.text_document_position.position.line as i32,
                    character: params.text_document_position.position.character as i32,
                },
                multiline: params.multiline,
            },
            parameters: SamplingParameters {
                max_new_tokens: params.parameters.max_new_tokens as usize,
                temperature: Option::from(params.parameters.temperature),
                top_p: None,
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false
        }
    }

    pub async fn get_completions(&self, params: CompletionParams1) -> Result<CompletionRes> {
        let mut post = self.flat_params_to_code_completion_post(&params).await;

        let res = handle_v1_code_completion(self.gcx.clone(),
                                            &mut post).await;
        let resp = res.unwrap();
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();

        let s = String::from_utf8(body_bytes.to_vec()).unwrap();
        let value = serde_json::from_str::<CompletionRes>(s.as_str()).map_err(|e| internal_error(e))?;

        Ok(value)
    }

    pub async fn test_if_head_tail_equal_return_added_text(&self, params: TestHeadTailAddedText) -> Result<TestHeadTailAddedTextRes> {
        let (is_valid, grey_corrected) = telemetry::utils::if_head_tail_equal_return_added_text(
            &params.text_a, &params.text_b, &params.orig_grey_text
        );
        let mut unchanged_percentage = -1.;
        if is_valid {
            unchanged_percentage = telemetry::utils::unchanged_percentage(
                &params.orig_grey_text,
                &grey_corrected
            );
        }
        Ok(TestHeadTailAddedTextRes{is_valid, grey_corrected, unchanged_percentage})
    }
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
                name: "refact".to_owned(),
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
        let rope = ropey::Rope::from_str(&params.content_changes[0].text);
        let uri = params.text_document.uri.to_string();
        let mut document_map = self.document_map.write().await;
        let doc = document_map
            .entry(uri.clone())
            .or_insert(Document::new("unknown".to_owned(), Rope::new()));
        doc.text = rope;
        info!("{} changed, save time: {:?}", uri, t0.elapsed());
        let t1 = Instant::now();
        telemetry::snippets_collection::sources_changed(
            self.gcx.clone(),
            &uri,
            &params.content_changes[0].text,
        ).await;
        info!("{} changed, telemetry time: {:?}", uri, t1.elapsed());
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file saved")
            .await;
        let uri = params.text_document.uri.to_string();
        info!("{uri} saved");
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file closed")
            .await;
        let uri = params.text_document.uri.to_string();
        info!("{uri} closed");
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = info!("shutdown");
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("LSP asked for completion");
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }
}

fn build_lsp_service(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> (LspService::<Backend>, ClientSocket) {
    let (lsp_service, socket) = LspService::build(|client| Backend {
        gcx,
        client,
        document_map: Arc::new(ARwLock::new(HashMap::new())),
        workspace_folders: Arc::new(ARwLock::new(None)),
    })
        .custom_method("refact/getCompletions", Backend::get_completions)
        .custom_method("refact/test_if_head_tail_equal_return_added_text", Backend::test_if_head_tail_equal_return_added_text)
        .finish();
    (lsp_service, socket)
}

pub fn spawn_lsp_task(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    cmdline: CommandLine
) -> Option<JoinHandle<()>> {
    if cmdline.lsp_stdin_stdout == 0 && cmdline.lsp_port > 0 {
        let gcx_t = gcx.clone();
        let addr: std::net::SocketAddr = ([127, 0, 0, 1], cmdline.lsp_port).into();
        return Some(tokio::spawn( async move {
            let listener: TcpListener = TcpListener::bind(&addr).await.unwrap();
            info!("LSP listening on {}", listener.local_addr().unwrap());
            loop {
                // possibly wrong code, look at
                // tower-lsp-0.20.0/examples/tcp.rs
                match listener.accept().await {
                    Ok((s, addr)) => {
                        info!("LSP new client connection from {}", addr);
                        let (read, write) = tokio::io::split(s);
                        let (lsp_service, socket) = build_lsp_service(gcx_t.clone());
                        tower_lsp::Server::new(read, write, socket).serve(lsp_service).await;
                    }
                    Err(e) => {
                        error!("Error accepting client connection: {}", e);
                    }
                }
            }
        }));
    }

    if cmdline.lsp_stdin_stdout != 0 && cmdline.lsp_port == 0 {
        let gcx_t = gcx.clone();
        return Some(tokio::spawn( async move {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let (lsp_service, socket) = build_lsp_service(gcx_t.clone());
            tower_lsp::Server::new(stdin, stdout, socket).serve(lsp_service).await;
            info!("LSP loop exit");
            gcx_t.write().await.ask_shutdown_sender.lock().unwrap().send(format!("going-down-because-lsp-exited")).unwrap();
        }));
    }

    None
}
