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
use crate::http::routers::v1::ast::{handle_v1_ast_cursor_search, handle_v1_ast_query_search};
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::chat::handle_v1_chat;
use crate::http::routers::v1::code_completion::handle_v1_code_completion_web;
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::snippet_accepted::handle_v1_snippet_accepted;
use crate::http::routers::v1::telemetry_network::handle_v1_telemetry_network;
use crate::http::routers::v1::lsp_like_handlers::handle_v1_lsp_initialize;
use crate::http::routers::v1::lsp_like_handlers::handle_v1_lsp_did_change;
use crate::http::utils::telemetry_wrapper;
use crate::http::routers::v1::vecdb::{handle_v1_vecdb_search, handle_v1_vecdb_status, handle_v1_vecdb_caps};
use crate::http::routers::v1::at_commands::{handle_v1_command_completion, handle_v1_command_preview};

pub mod code_completion;
pub mod chat;
pub mod telemetry_network;
pub mod snippet_accepted;
pub mod caps;
pub mod graceful_shutdown;
pub mod lsp_like_handlers;
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

        .route("/ast-cursor-search", telemetry_post!(handle_v1_ast_cursor_search))
        .route("/ast-query-search", telemetry_post!(handle_v1_ast_query_search))
}