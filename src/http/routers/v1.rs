use std::pin::Pin;

use axum::Extension;
use axum::Router;
use axum::routing::get;
use axum::routing::post;
use futures::Future;
use hyper::Body;
use hyper::Response;

use crate::{telemetry_get, telemetry_post};
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::http::routers::v1::ast::{handle_v1_ast_declarations_cursor_search, handle_v1_ast_declarations_query_search,
                                    handle_v1_ast_references_cursor_search, handle_v1_ast_references_query_search,
                                    handle_v1_ast_file_symbols, handle_v1_ast_index_file,
                                    handle_v1_ast_clear_index};
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::chat::handle_v1_chat;
use crate::http::routers::v1::code_completion::handle_v1_code_completion_web;
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::snippet_accepted::handle_v1_snippet_accepted;
use crate::http::routers::v1::telemetry_network::handle_v1_telemetry_network;
use crate::http::routers::v1::lsp_like_handlers::{handle_v1_lsp_add_folder, handle_v1_lsp_initialize, handle_v1_lsp_remove_folder};
use crate::http::routers::v1::lsp_like_handlers::handle_v1_lsp_did_change;
use crate::http::routers::v1::toolbox::handle_v1_customization;
use crate::http::routers::v1::toolbox::handle_v1_rewrite_assistant_says_to_at_commands;
use crate::http::utils::telemetry_wrapper;
use crate::http::routers::v1::dashboard::get_dashboard_plots;
use crate::http::routers::v1::vecdb::{handle_v1_vecdb_search, handle_v1_vecdb_status, handle_v1_vecdb_caps};
use crate::http::routers::v1::at_commands::{handle_v1_command_completion, handle_v1_command_preview};

pub mod code_completion;
pub mod chat;
pub mod telemetry_network;
pub mod snippet_accepted;
pub mod caps;
pub mod graceful_shutdown;
mod dashboard;
pub mod lsp_like_handlers;
pub mod toolbox;
pub mod vecdb;
mod at_commands;
mod ast;

pub fn make_v1_router() -> Router {
    Router::new()
        .route("/code-completion", telemetry_post!(handle_v1_code_completion_web))
        .route("/chat", telemetry_post!(handle_v1_chat))
        .route("/telemetry-network", telemetry_post!(handle_v1_telemetry_network))
        .route("/snippet-accepted", telemetry_post!(handle_v1_snippet_accepted))

        .route("/caps", telemetry_get!(handle_v1_caps))
        .route("/graceful-shutdown", telemetry_get!(handle_v1_graceful_shutdown))

        .route("/vdb-search", telemetry_post!(handle_v1_vecdb_search))
        .route("/vdb-status", telemetry_get!(handle_v1_vecdb_status))
        .route("/vdb-caps", telemetry_get!(handle_v1_vecdb_caps))
        .route("/at-command-completion", telemetry_post!(handle_v1_command_completion))
        .route("/at-command-preview", telemetry_post!(handle_v1_command_preview))

        .route("/lsp-initialize", telemetry_post!(handle_v1_lsp_initialize))
        .route("/lsp-did-changed", telemetry_post!(handle_v1_lsp_did_change))
        .route("/lsp-add-folder", telemetry_post!(handle_v1_lsp_add_folder))
        .route("/lsp-remove-folder", telemetry_post!(handle_v1_lsp_remove_folder))

        .route("/get-dashboard-plots", telemetry_get!(get_dashboard_plots))

        .route("/ast-declarations-cursor-search", telemetry_post!(handle_v1_ast_declarations_cursor_search))
        .route("/ast-declarations-query-search", telemetry_post!(handle_v1_ast_declarations_query_search))
        .route("/ast-references-cursor-search", telemetry_post!(handle_v1_ast_references_cursor_search))
        .route("/ast-references-query-search", telemetry_post!(handle_v1_ast_references_query_search))
        .route("/ast-file-symbols", telemetry_post!(handle_v1_ast_file_symbols))
        .route("/ast-index-file", telemetry_post!(handle_v1_ast_index_file))
        .route("/ast-clear-index", telemetry_post!(handle_v1_ast_clear_index))

        // experimental
        .route("/customization", telemetry_get!(handle_v1_customization))
        .route("/rewrite-assistant-says-to-at-commands", telemetry_post!(handle_v1_rewrite_assistant_says_to_at_commands))
}
