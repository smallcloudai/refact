use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use std::io::Write;

use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tower_lsp::{ClientSocket, LanguageServer, LspService};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tracing::{error, info};

use crate::call_validation::{CodeCompletionInputs, CodeCompletionPost, CursorPosition, SamplingParameters};
use crate::files_in_workspace;
use crate::files_in_workspace::{on_did_change, on_did_delete};
use crate::global_context::{CommandLine, GlobalContext};
use crate::http::routers::v1::code_completion::handle_v1_code_completion;
use crate::telemetry::snippets_collection;

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


pub struct LspBackend {
    pub gcx: Arc<ARwLock<GlobalContext>>,
    pub client: tower_lsp::Client,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct SnippetAcceptedParams {
    snippet_telemetry_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeActiveFile {
    pub uri: Url,
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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SuccessRes {
    pub success: bool,
}

impl LspBackend {
    async fn flat_params_to_code_completion_post(&self, params: &CompletionParams1) -> Result<CodeCompletionPost> {
        let path = crate::files_correction::canonical_path(&params.text_document_position.text_document.uri.to_file_path().unwrap_or_default().display().to_string());
        let txt = match self.gcx.read().await.documents_state.memory_document_map.get(&path) {
            Some(doc) => doc.read().await.clone().get_text_or_read_from_disk(self.gcx.clone()).await.unwrap_or_default(),
            None => return Err(internal_error("document not found"))
        };
        // url -> String method should be the same as in telemetry::snippets_collection::sources_changed
        let path_string = params.text_document_position.text_document.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string();
        Ok(CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from([(path_string.clone(), (&txt).to_string())]),
                cursor: CursorPosition {
                    file: path_string.clone(),
                    line: params.text_document_position.position.line as i32,
                    character: params.text_document_position.position.character as i32,
                },
                multiline: params.multiline,
            },
            parameters: SamplingParameters {
                max_new_tokens: params.parameters.max_new_tokens as usize,
                temperature: Option::from(params.parameters.temperature),
                ..Default::default()
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: false,
            use_vecdb: false,
            rag_tokens_n: 0,
        })
    }

    pub async fn get_completions(&self, params: CompletionParams1) -> Result<CompletionRes> {
        let mut post = self.flat_params_to_code_completion_post(&params).await?;

        let res = handle_v1_code_completion(self.gcx.clone(), &mut post)
            .await.map_err(|e| internal_error(e))?;

        let body_bytes = hyper::body::to_bytes(res.into_body()).await.map_err(|e| internal_error(e))?;

        let s = String::from_utf8(body_bytes.to_vec()).map_err(|e|internal_error(e))?;
        let value = serde_json::from_str::<CompletionRes>(s.as_str()).map_err(|e| internal_error(e))?;

        Ok(value)
    }

    pub async fn accept_snippet(&self, params: SnippetAcceptedParams) -> Result<SuccessRes> {
        let success = snippets_collection::snippet_accepted(self.gcx.clone(), params.snippet_telemetry_id).await;
        Ok(SuccessRes { success })
    }

    pub async fn set_active_document(&self, params: ChangeActiveFile) -> Result<SuccessRes> {
        let path = crate::files_correction::canonical_path(&params.uri.to_file_path().unwrap_or_default().display().to_string());
        info!("ACTIVE_DOC {:?}", crate::nicer_logs::last_n_chars(&path.to_string_lossy().to_string(), 30));
        self.gcx.write().await.documents_state.active_file_path = Some(path);
        Ok(SuccessRes { success: true })
    }

    async fn ping_http_server(&self) -> Result<()> {
        let (port, http_client) = {
            let gcx_locked = self.gcx.write().await;
            (gcx_locked.cmdline.http_port, gcx_locked.http_client.clone())
        };

        let url = "http://127.0.0.1:".to_string() + &port.to_string() + &"/v1/ping".to_string();
        let mut attempts = 0;
        while attempts < 15 {
            let response = http_client.get(&url).send().await;
            match response {
                Ok(res) if res.status().is_success() => {
                    return Ok(());
                }
                _ => {
                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                }
            }
        }
        Err(internal_error("HTTP server is not ready after 15 attempts"))
    }
 }


#[tower_lsp::async_trait]
impl LanguageServer for LspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("LSP client_info {:?}", params.client_info);
        let mut folders: Vec<PathBuf> = vec![];
        if let Some(nonzero_folders) = params.workspace_folders {
            folders = nonzero_folders.iter().map(|x| {
                let path = crate::files_correction::canonical_path(&x.uri.to_file_path().unwrap_or_default().display().to_string());
                path
            }).collect();
        }
        {
            let gcx_locked = self.gcx.write().await;
            *gcx_locked.documents_state.workspace_folders.lock().unwrap() = folders.clone();
            info!("LSP workspace_folders {:?}", folders);
        }
        files_in_workspace::on_workspaces_init(
            self.gcx.clone(),
        ).await;

        let completion_options: CompletionOptions;
        completion_options = CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".(".to_owned()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions { work_done_progress: Some(false) },
            completion_item: None,
        };
        let file_filter = FileOperationRegistrationOptions {
            filters: vec![FileOperationFilter {
                scheme: None,
                pattern: FileOperationPattern {
                    glob: "**".to_string(),
                    matches: None,
                    options: None,
                }
            }],
        };


        // wait for http server to be ready
        self.ping_http_server().await?;

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
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                        did_create: Some(file_filter.clone()),
                        will_create: Some(file_filter.clone()),
                        did_rename: Some(file_filter.clone()),
                        will_rename: Some(file_filter.clone()),
                        did_delete: Some(file_filter.clone()),
                        will_delete: Some(file_filter.clone()),
                    }),
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rust LSP received initialized()")
            .await;
        let _ = info!("rust LSP received initialized()");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let cpath = crate::files_correction::canonical_path(&params.text_document.uri.to_file_path().unwrap_or_default().display().to_string());
        if cpath.to_string_lossy().contains("keybindings.json") {
            return;
        }
        files_in_workspace::on_did_open(
            self.gcx.clone(),
            &cpath,
            &params.text_document.text,
            &params.text_document.language_id
        ).await
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file closed")
            .await;
        let cpath = crate::files_correction::canonical_path(&params.text_document.uri.to_file_path().unwrap_or_default().display().to_string());
        files_in_workspace::on_did_close(
            self.gcx.clone(),
            &cpath,
        ).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let path = crate::files_correction::canonical_path(&params.text_document.uri.to_file_path().unwrap_or_default().display().to_string());
        on_did_change(
            self.gcx.clone(),
            &path,
            &params.content_changes[0].text  // TODO: This text could be just a part of the whole file (if range is not none)
        ).await
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let path = crate::files_correction::canonical_path(&params.text_document.uri.to_file_path().unwrap_or_default().display().to_string());
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file saved")
            .await;
        info!("{} saved", path.display());
    }

    async fn shutdown(&self) -> Result<()> {
        info!("shutdown");
        self.gcx.write().await.ask_shutdown_sender.lock().unwrap().send("LSP SHUTDOWN".to_string()).unwrap();
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("LSP asked for popup completions");
        Ok(Some(CompletionResponse::Array(vec![])))
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        for folder in params.event.added {
            info!("did_change_workspace_folders/add {}", folder.name);
            let path = crate::files_correction::canonical_path(&folder.uri.to_file_path().unwrap_or_default().display().to_string());
            files_in_workspace::add_folder(self.gcx.clone(), &path).await;
        }
        for folder in params.event.removed {
            info!("did_change_workspace_folders/delete {}", folder.name);
            let path = crate::files_correction::canonical_path(&folder.uri.to_file_path().unwrap_or_default().display().to_string());
            files_in_workspace::remove_folder(self.gcx.clone(), &path).await;
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for event in params.changes {
            if event.typ == FileChangeType::DELETED {
                let cpath = crate::files_correction::canonical_path(&event.uri.to_file_path().unwrap_or_default().display().to_string());
                info!("UNCLEAR LSP EVENT: did_change_watched_files/delete {}", cpath.display());
                on_did_delete(self.gcx.clone(), &cpath).await;
            }
            else if event.typ == FileChangeType::CREATED {
                let cpath = crate::files_correction::canonical_path(&event.uri.to_file_path().unwrap_or_default().display().to_string());
                info!("UNCLEAR LSP EVENT: did_change_watched_files/change {}", cpath.display());
                // on_did_change(self.gcx.clone(), &cpath, &text).await;
            }
        }
    }
}

async fn build_lsp_service(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> (LspService::<LspBackend>, ClientSocket) {
    let (lsp_service, socket) = LspService::build(|client| LspBackend {
        gcx,
        client,
    })
        .custom_method("refact/getCompletions", LspBackend::get_completions)
        .custom_method("refact/acceptCompletion", LspBackend::accept_snippet)
        .custom_method("refact/setActiveDocument", LspBackend::set_active_document)
        .finish();
    (lsp_service, socket)
}

pub async fn spawn_lsp_task(
    gcx: Arc<ARwLock<GlobalContext>>,
    cmdline: CommandLine
) -> Option<JoinHandle<()>> {
    if cmdline.lsp_stdin_stdout == 0 && cmdline.lsp_port > 0 {
        let gcx_t = gcx.clone();
        let addr: std::net::SocketAddr = ([127, 0, 0, 1], cmdline.lsp_port).into();
        return Some(tokio::spawn(async move {
            let listener_maybe = TcpListener::bind(&addr).await;
            if listener_maybe.is_err() {
                let _ = write!(std::io::stderr(), "PORT_BUSY\n{}: {}\n", addr, listener_maybe.unwrap_err());
                gcx_t.write().await.ask_shutdown_sender.lock().unwrap().send("LSP PORT_BUSY".to_string()).unwrap();
                return;
            }
            let listener = listener_maybe.unwrap();
            info!("LSP listening on {}", listener.local_addr().unwrap());
            loop {
                // possibly wrong code, look at
                // tower-lsp-0.20.0/examples/tcp.rs
                match listener.accept().await {
                    Ok((s, addr)) => {
                        info!("LSP new client connection from {}", addr);
                        let (read, write) = tokio::io::split(s);
                        let (lsp_service, socket) = build_lsp_service(gcx_t.clone()).await;
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
        return Some(tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let (lsp_service, socket) = build_lsp_service(gcx_t.clone()).await;
            tower_lsp::Server::new(stdin, stdout, socket).serve(lsp_service).await;
            info!("LSP loop exit");
            match gcx_t.write().await.ask_shutdown_sender.lock() {
                Ok(sender) => {
                    if let Err(err) = sender.send("going-down-because-lsp-exited".to_string()) {
                        error!("Failed to send shutdown message: {}", err);
                    }
                }
                Err(err) => {
                    error!("Failed to lock ask_shutdown_sender: {}", err);
                }
            }
        }));
    }

    None
}
