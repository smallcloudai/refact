use ropey::Rope;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
// use std::time::Instant;
// use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error, info};


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
    // cache_dir: PathBuf,
    pub client: Client,
    pub document_map: Arc<RwLock<HashMap<String, Document>>>,
    // http_client: reqwest::Client,
    // unsafe_http_client: reqwest::Client,
    pub workspace_folders: Arc<RwLock<Option<Vec<WorkspaceFolder>>>>,
    // tokenizer_map: Arc<RwLock<HashMap<String, Arc<Tokenizer>>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Completion {
    pub generated_text: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ide {
    Neovim,
    VSCode,
    JetBrains,
    Emacs,
    Jupyter,
    Sublime,
    VisualStudio,
    #[default]
    Unknown,
}

impl Display for Ide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

fn parse_ide<'de, D>(d: D) -> std::result::Result<Ide, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).map(|b: Option<_>| b.unwrap_or(Ide::Unknown))
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestParams {
    pub max_new_tokens: u32,
    pub temperature: f32,
    pub do_sample: bool,
    pub top_p: f32,
    pub stop_tokens: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionParams1 {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
    pub request_params: RequestParams,
    #[serde(default)]
    #[serde(deserialize_with = "parse_ide")]
    pub ide: Ide,
    // fim: FimParams,
    pub api_token: Option<String>,
    pub model: String,
    pub tokens_to_clear: Vec<String>,
    // tokenizer_config: Option<TokenizerConfig>,
    pub context_window: usize,
    pub tls_skip_verify_insecure: bool,
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

// async fn request_completion(
//     http_client: &reqwest::Client,
//     ide: Ide,
//     model: &str,
//     request_params: RequestParams,
//     api_token: Option<&String>,
//     prompt: String,
// ) -> Result<Vec<Generation>> {
//     let res = http_client
//         .post(build_url(model))
//         .json(&APIRequest {
//             inputs: prompt,
//             parameters: request_params.into(),
//         })
//         .headers(build_headers(api_token, ide)?)
//         .send()
//         .await
//         .map_err(internal_error)?;

//     match res.json().await.map_err(internal_error)? {
//         APIResponse::Generation(gen) => Ok(vec![gen]),
//         APIResponse::Generations(gens) => Ok(gens),
//         APIResponse::Error(err) => Err(internal_error(err)),
//     }
// }

impl Backend {
    pub async fn get_completions(&self, params: CompletionParams1) -> Result<Vec<Completion>> {
        info!("get_completions {params:?}");
        let document_map = self.document_map.read().await;

        let document = document_map
            .get(params.text_document_position.text_document.uri.as_str())
            .ok_or_else(|| internal_error("failed to find document"))?;
        info!("document: {:?}", document);
        // let tokenizer = get_tokenizer(
        //     &params.model,
        //     &mut *self.tokenizer_map.write().await,
        //     params.tokenizer_config,
        //     &self.http_client,
        //     &self.cache_dir,
        //     params.api_token.as_ref(),
        //     params.ide,
        // )
        // .await?;
        // let prompt = build_prompt(
        //     params.text_document_position.position,
        //     &document.text,
        //     &params.fim,
        //     tokenizer,
        //     params.context_window,
        // )?;

        // let http_client = if params.tls_skip_verify_insecure {
        //     info!("tls verification is disabled");
        //     &self.unsafe_http_client
        // } else {
        //     &self.http_client
        // };
        // let result = request_completion(
        //     http_client,
        //     params.ide,
        //     &params.model,
        //     params.request_params,
        //     params.api_token.as_ref(),
        //     prompt,
        // )
        // .await?;

        Ok(vec![Completion { generated_text: "hello".to_owned() }])
    }
}


// pub struct CompletionOptions {
//     /// The server provides support to resolve additional information for a completion item.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub resolve_provider: Option<bool>,

//     /// Most tools trigger completion request automatically without explicitly
//     /// requesting it using a keyboard shortcut (e.g. Ctrl+Space). Typically they
//     /// do so when the user starts to type an identifier. For example if the user
//     /// types `c` in a JavaScript file code complete will automatically pop up
//     /// present `console` besides others as a completion item. Characters that
//     /// make up identifiers don't need to be listed here.
//     ///
//     /// If code complete should automatically be trigger on characters not being
//     /// valid inside an identifier (for example `.` in JavaScript) list them in
//     /// `triggerCharacters`.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub trigger_characters: Option<Vec<String>>,

//     /// The list of all possible characters that commit a completion. This field
//     /// can be used if clients don't support individual commit characters per
//     /// completion item. See client capability
//     /// `completion.completionItem.commitCharactersSupport`.
//     ///
//     /// If a server provides both `allCommitCharacters` and commit characters on
//     /// an individual completion item the ones on the completion item win.
//     ///
//     /// @since 3.2.0
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub all_commit_characters: Option<Vec<String>>,

//     #[serde(flatten)]
//     pub work_done_progress_options: WorkDoneProgressOptions,

//     /// The server supports the following `CompletionItem` specific
//     /// capabilities.
//     ///
//     /// @since 3.17.0
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub completion_item: Option<CompletionOptionsCompletionItem>,
// }

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        *self.workspace_folders.write().await = params.workspace_folders;
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
        info!("{uri} changed");
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
        let _ = debug!("shutdown");
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

// fn build_headers(api_token: Option<&String>, ide: Ide) -> Result<HeaderMap> {
//     let mut headers = HeaderMap::new();
//     let user_agent = format!("{NAME}/{VERSION}; rust/unknown; ide/{ide:?}");
//     headers.insert(
//         USER_AGENT,
//         HeaderValue::from_str(&user_agent).map_err(internal_error)?,
//     );

//     if let Some(api_token) = api_token {
//         headers.insert(
//             AUTHORIZATION,
//             HeaderValue::from_str(&format!("Bearer {api_token}")).map_err(internal_error)?,
//         );
//     }

//     Ok(headers)
// }

// #[tokio::main]
// async fn main() {
//     let stdin = tokio::io::stdin();
//     let stdout = tokio::io::stdout();

//     let home_dir = home::home_dir().ok_or(()).expect("failed to find home dir");
//     let cache_dir = home_dir.join(".cache/llm_ls");
//     tokio::fs::create_dir_all(&cache_dir)
//         .await
//         .expect("failed to create cache dir");

//     let log_file = rolling::never(&cache_dir, "llm-ls.log");
//     let builder = tracing_subscriber::fmt()
//         .with_writer(log_file)
//         .with_target(true)
//         .with_line_number(true)
//         .with_env_filter(
//             EnvFilter::try_from_env("LLM_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("warn")),
//         );

//     builder
//         .json()
//         .flatten_event(true)
//         .with_current_span(false)
//         .with_span_list(true)
//         .init();

//     let http_client = reqwest::Client::new();
//     let unsafe_http_client = reqwest::Client::builder()
//         .danger_accept_invalid_certs(true)
//         .build()
//         .expect("failed to build reqwest unsafe client");

//     let (service, socket) = LspService::build(|client| Backend {
//         cache_dir,
//         client,
//         document_map: Arc::new(RwLock::new(HashMap::new())),
//         http_client,
//         unsafe_http_client,
//         workspace_folders: Arc::new(RwLock::new(None)),
//         tokenizer_map: Arc::new(RwLock::new(HashMap::new())),
//     })
//     .custom_method("llm-ls/getCompletions", Backend::get_completions)
//     .finish();

//     Server::new(stdin, stdout, socket).serve(service).await;
// }
