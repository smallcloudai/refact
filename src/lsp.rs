use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

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
use crate::files_in_workspace::on_did_delete;
use crate::global_context;
use crate::global_context::CommandLine;
use crate::http::routers::v1::code_completion::handle_v1_code_completion;
use crate::telemetry;
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


// #[derive(Debug)]  GlobalContext does not implement Debug
pub struct Backend {
    pub gcx: Arc<ARwLock<global_context::GlobalContext>>,
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

impl Backend {
    async fn flat_params_to_code_completion_post(&self, params: &CompletionParams1) -> Result<CodeCompletionPost> {
        let txt = {
            let document_map = self.gcx.read().await.documents_state.document_map.clone();  // Arc::ARwLock
            let document_map = document_map.read().await;
            let document = document_map.get(&params.text_document_position.text_document.uri);
            match document {
                None => {
                    return Err(internal_error("document not found"));
                }
                Some(doc) => {
                    doc.text.clone()
                }
            }
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
                top_p: None,
                stop: None,
            },
            model: "".to_string(),
            scratchpad: "".to_string(),
            stream: false,
            no_cache: false,
            use_ast: false,
            use_vecdb: false,
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

    pub async fn test_if_head_tail_equal_return_added_text(&self, params: TestHeadTailAddedText) -> Result<TestHeadTailAddedTextRes> {
        let (is_valid, grey_corrected) = telemetry::utils::if_head_tail_equal_return_added_text(
            &params.text_a, &params.text_b, &params.orig_grey_text
        );
        let mut unchanged_percentage = -1.;
        if is_valid {
            unchanged_percentage = telemetry::utils::unchanged_percentage(
                &params.orig_grey_text,
                &grey_corrected,
            );
        }
        Ok(TestHeadTailAddedTextRes { is_valid, grey_corrected, unchanged_percentage })
    }
 }


#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("LSP client_info {:?}", params.client_info);
        let mut folders: Vec<PathBuf> = vec![];
        if let Some(nonzero_folders) = params.workspace_folders {
            folders = nonzero_folders.iter().map(|x| PathBuf::from(x.uri.path())).collect();
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
        files_in_workspace::on_did_open(
            self.gcx.clone(),
            &params.text_document.uri,
            &params.text_document.text,
            &params.text_document.language_id
        ).await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        files_in_workspace::on_did_change(
            self.gcx.clone(),
            &params.text_document.uri,
            &params.content_changes[0].text  // TODO: This text could be just a part of the whole file
        ).await
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
        info!("LSP asked for popup completions");
        Ok(Some(CompletionResponse::Array(vec![])))
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        for _add_folder in params.event.added {
            // TODO
        }
        for _delete_folder in params.event.removed {
            // TODO
        }
    }
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for event in params.changes {
            let uri = event.uri;
            if event.typ == FileChangeType::DELETED {
                on_did_delete(self.gcx.clone(), &uri).await;
            }
        }
    }
}

async fn build_lsp_service(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
) -> (LspService::<Backend>, ClientSocket) {
    let (lsp_service, socket) = LspService::build(|client| Backend {
        gcx,
        client,
    })
        .custom_method("refact/getCompletions", Backend::get_completions)
        .custom_method("refact/acceptCompletion", Backend::accept_snippet)
        .custom_method("refact/test_if_head_tail_equal_return_added_text", Backend::test_if_head_tail_equal_return_added_text)
        .finish();
    (lsp_service, socket)
}

pub async fn spawn_lsp_task(
    gcx: Arc<ARwLock<global_context::GlobalContext>>,
    cmdline: CommandLine
) -> Option<JoinHandle<()>> {
    if cmdline.lsp_stdin_stdout == 0 && cmdline.lsp_port > 0 {
        let gcx_t = gcx.clone();
        let addr: std::net::SocketAddr = ([127, 0, 0, 1], cmdline.lsp_port).into();
        return Some(tokio::spawn(async move {
            let listener: TcpListener = TcpListener::bind(&addr).await.unwrap();
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
            gcx_t.write().await.ask_shutdown_sender.lock().unwrap().send(format!("going-down-because-lsp-exited")).unwrap();
        }));
    }

    None
}
