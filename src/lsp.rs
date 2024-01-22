use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

use std::time::Instant;
use tracing::log::warn;
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
use crate::lsp::document::Document;

use crate::telemetry;
use crate::receive_workspace_changes;
use crate::telemetry;
use crate::vecdb::file_filter::is_valid_file;

use crate::telemetry::snippets_collection::sources_changed;

mod treesitter;
mod language_id;
pub mod document;

const VERSION: &str = env!("CARGO_PKG_VERSION");


#[derive(Debug, Deserialize)]
struct APIError {
    error: String,
}

impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}


// #[derive(Debug)]
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
    async fn flat_params_to_code_completion_post(&self, params: &CompletionParams1) -> Result<CodeCompletionPost> {
        // let txt = {
            // let document_map = self.gcx.read().await.lsp_backend_document_state.document_map.clone();
            // let document_map = document_map.read().await;
            // let document = document_map
            //     .get(params.text_document_position.text_document.uri.as_str());
            // match document {
            //     None => {
            //         return Err(internal_error("document not found"));
            //     }
            //     Some(doc) => {
            //         doc.text.clone()
            //     }
            // }
        // };
        Ok(CodeCompletionPost {
            inputs: CodeCompletionInputs {
                sources: HashMap::from([(String::from(&params.text_document_position.text_document.uri.to_string()),
                                         (&"").to_string())]),
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
        {
            let gcx_locked = self.gcx.write().await;
            // *gcx_locked.lsp_backend_document_state.workspace_folders.write().await = params.workspace_folders.clone();
            // info!("LSP workspace_folders {:?}", gcx_locked.lsp_backend_document_state.workspace_folders);
        }

        if let Some(folders) = params.workspace_folders {
            match *self.gcx.read().await.vec_db.lock().await {
                Some(ref mut db) => db.init_folders(folders).await,
                None => {},
            };
        }

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

    async fn shutdown(&self) -> Result<()> {
        let _ = info!("shutdown");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        receive_workspace_changes::on_did_open(
            self.gcx.clone(),
            &params.text_document.uri.to_string(),
            &params.text_document.text,
            &params.text_document.language_id
        ).await;

        let uri = params.text_document.uri.to_string();
        match Document::open(
            &params.text_document.language_id,
            &params.text_document.text,
            &params.text_document.uri.to_string()
        ) {
            Ok(document) => {
                // let gc = self.gcx.clone();
                // let gx = gc.write().await;
                // gx.lsp_backend_document_state.document_map
                //     .write()
                //     .await
                //     .insert(uri.clone(), document);
                info!("{uri} opened");
            }
            Err(err) => error!("error opening {uri}: {err}"),
        }
        info!("{uri} opened");
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let file_path = PathBuf::from(params.text_document.uri.path());
        if is_valid_file(&file_path) {
            match *self.gcx.read().await.vec_db.lock().await {
                Some(ref mut db) => db.add_or_update_file(file_path, false).await,
                None => {}
            };
        }

        receive_workspace_changes::on_did_change(
            self.gcx.clone(),
            &params.text_document.uri.to_string(),
            &params.content_changes[0].text
        ).await;

        let t0 = Instant::now();
        let uri = params.text_document.uri.to_string();
        let gc = self.gcx.clone();
        let gx = gc.write().await;
        // let mut document_map = gx.lsp_backend_document_state.document_map.write().await;
        // let doc = document_map.get_mut(&uri);
        // if let Some(doc) = doc {
        //     match doc.change(&params.content_changes[0].text).await {
        //         Ok(()) => {
        //             info!("{} changed, save time: {:?}", uri, t0.elapsed());
        //             let t1 = Instant::now();
        //             sources_changed(
        //                 self.gcx.clone(),
        //                 &uri,
        //                 &params.content_changes[0].text,
        //             ).await;
        //             info!("{} changed, telemetry time: {:?}", uri, t1.elapsed());
        //         },
        //         Err(err) => error!("error when changing {uri}: {err}"),
        //     }
        // } else {
        //     warn!("textDocument/didChange {uri}: document not found");
        // }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file saved")
            .await;
        let uri = params.text_document.uri.to_string();
        let file_path = PathBuf::from(params.text_document.uri.path());
        if is_valid_file(&file_path) {
            match *self.gcx.read().await.vec_db.lock().await {
                Some(ref mut db) => db.add_or_update_file(file_path, false).await,
                None => {}
            };
        }
        info!("{uri} saved");
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "{refact-lsp} file closed")
            .await;
        let uri = params.text_document.uri.to_string();
        let file_path = PathBuf::from(params.text_document.uri.path());
        if is_valid_file(&file_path) {
            match *self.gcx.read().await.vec_db.lock().await {
                Some(ref mut db) => db.add_or_update_file(file_path, false).await,
                None => {}
            };
        }
        info!("{uri} closed");
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = info!("shutdown");
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!("LSP asked for popup completions");
        Ok(Some(CompletionResponse::Array(vec![
        ])))
    }

    async fn did_delete_files(&self, params: DeleteFilesParams) {
        let files = params.files
            .into_iter()
            .map(|x| PathBuf::from(x.uri.replace("file://", "")))
            .filter(|x| is_valid_file(&x));

        match *self.gcx.read().await.vec_db.lock().await {
            Some(ref mut db) => {
                for file in files {
                    db.remove_file(&file).await;
                }
            }
            None => {}
        };
    }

    async fn did_create_files(&self, params: CreateFilesParams) {
        let files = params.files
            .into_iter()
            .map(|x| PathBuf::from(x.uri.replace("file://", "")))
            .filter(|x| is_valid_file(&x))
            .collect();
        match *self.gcx.read().await.vec_db.lock().await {
            Some(ref db) => db.add_or_update_files(files, false).await,
            None => {}
        };
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
