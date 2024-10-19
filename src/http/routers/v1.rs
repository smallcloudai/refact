use std::pin::Pin;
use axum::Extension;
use axum::Router;
use axum::routing::get;
use axum::routing::post;
use futures::Future;
use hyper::Body;
use hyper::Response;
use tower_http::cors::CorsLayer;

use crate::{telemetry_get, telemetry_post};
use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::http::routers::v1::code_completion::{handle_v1_code_completion_web, handle_v1_code_completion_prompt};
use crate::http::routers::v1::code_lens::handle_v1_code_lens;
use crate::http::routers::v1::ast::{handle_v1_ast_file_dump, handle_v1_ast_file_symbols, handle_v1_ast_status};
use crate::http::routers::v1::at_commands::{handle_v1_command_completion, handle_v1_command_preview};
use crate::http::routers::v1::at_tools::{handle_v1_tools, handle_v1_tools_check_if_confirmation_needed};
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::caps::handle_v1_ping;
use crate::http::routers::v1::chat::{handle_v1_chat, handle_v1_chat_completions};
use crate::http::routers::v1::dashboard::get_dashboard_plots;
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::snippet_accepted::handle_v1_snippet_accepted;
use crate::http::routers::v1::telemetry_network::handle_v1_telemetry_network;
use crate::http::routers::v1::lsp_like_handlers::{handle_v1_lsp_did_change, handle_v1_lsp_add_folder, handle_v1_lsp_initialize, handle_v1_lsp_remove_folder, handle_v1_set_active_document};
use crate::http::routers::v1::status::handle_v1_rag_status;
use crate::http::routers::v1::customization::handle_v1_customization;
use crate::http::routers::v1::customization::handle_v1_config_path;
use crate::http::routers::v1::gui_help_handlers::handle_v1_fullpath;
use crate::http::routers::v1::patch::handle_v1_patch_single_file_from_ticket;
use crate::http::routers::v1::subchat::{handle_v1_subchat, handle_v1_subchat_single};

#[cfg(feature="vecdb")]
use crate::http::routers::v1::vecdb::{handle_v1_vecdb_search, handle_v1_vecdb_status};
#[cfg(feature="vecdb")]
use crate::http::routers::v1::handlers_memdb::{handle_mem_query, handle_mem_add, handle_mem_erase, handle_mem_update_used, handle_mem_block_until_vectorized, handle_mem_list, handle_ongoing_update_or_create, handle_ongoing_dump};

use crate::http::utils::telemetry_wrapper;

pub mod code_completion;
pub mod code_lens;
pub mod chat;
pub mod telemetry_network;
pub mod snippet_accepted;
pub mod caps;
pub mod graceful_shutdown;
mod dashboard;
pub mod lsp_like_handlers;
pub mod customization;
mod at_commands;
mod ast;
mod at_tools;
mod status;

mod subchat;
mod gui_help_handlers;
mod patch;

#[cfg(feature="vecdb")]
pub mod handlers_memdb;
#[cfg(feature="vecdb")]
pub mod vecdb;


pub fn make_v1_router() -> Router {
    let builder = Router::new()
        .route("/ping", telemetry_get!(handle_v1_ping))

        .route("/code-completion", telemetry_post!(handle_v1_code_completion_web))
        .route("/code-lens", telemetry_post!(handle_v1_code_lens))

        .route("/chat", telemetry_post!(handle_v1_chat))
        .route("/chat/completions", telemetry_post!(handle_v1_chat_completions))  // standard
        .route("/telemetry-network", telemetry_post!(handle_v1_telemetry_network))
        .route("/snippet-accepted", telemetry_post!(handle_v1_snippet_accepted))

        .route("/caps", telemetry_get!(handle_v1_caps))
        .route("/graceful-shutdown", telemetry_get!(handle_v1_graceful_shutdown))

        .route("/at-command-completion", telemetry_post!(handle_v1_command_completion))
        .route("/at-command-preview", telemetry_post!(handle_v1_command_preview))

        .route("/tools", telemetry_get!(handle_v1_tools))
        .route("/tools-check-if-confirmation-needed", telemetry_post!(handle_v1_tools_check_if_confirmation_needed))

        .route("/lsp-initialize", telemetry_post!(handle_v1_lsp_initialize))
        .route("/lsp-did-changed", telemetry_post!(handle_v1_lsp_did_change))
        .route("/lsp-add-folder", telemetry_post!(handle_v1_lsp_add_folder))
        .route("/lsp-remove-folder", telemetry_post!(handle_v1_lsp_remove_folder))
        .route("/lsp-set-active-document", telemetry_post!(handle_v1_set_active_document))

        .route("/get-dashboard-plots", telemetry_get!(get_dashboard_plots))

        .route("/ast-file-symbols", telemetry_post!(handle_v1_ast_file_symbols))
        .route("/ast-file-dump", telemetry_post!(handle_v1_ast_file_dump))
        .route("/ast-status", telemetry_get!(handle_v1_ast_status))

        .route("/rag-status", telemetry_get!(handle_v1_rag_status))
        .route("/config-path", telemetry_get!(handle_v1_config_path))
        // experimental
        .route("/customization", telemetry_get!(handle_v1_customization))

        .route("/code-completion-prompt", telemetry_post!(handle_v1_code_completion_prompt))

        .route("/patch-single-file-from-ticket", telemetry_post!(handle_v1_patch_single_file_from_ticket))

        .route("/subchat", telemetry_post!(handle_v1_subchat))
        .route("/subchat-single", telemetry_post!(handle_v1_subchat_single))

        .route("/fullpath", telemetry_post!(handle_v1_fullpath));

    #[cfg(feature="vecdb")]
    let builder = builder
        .route("/vdb-search", telemetry_post!(handle_v1_vecdb_search))
        .route("/vdb-status", telemetry_get!(handle_v1_vecdb_status))
        .route("/mem-query", telemetry_post!(handle_mem_query))
        .route("/mem-add", telemetry_post!(handle_mem_add))
        .route("/mem-erase", telemetry_post!(handle_mem_erase))
        .route("/mem-update-used", telemetry_post!(handle_mem_update_used))
        .route("/mem-block-until-vectorized", telemetry_get!(handle_mem_block_until_vectorized))
        .route("/mem-list", telemetry_get!(handle_mem_list))
        .route("/ongoing-update", telemetry_post!(handle_ongoing_update_or_create))
        .route("/ongoing-dump", telemetry_get!(handle_ongoing_dump));

    builder.layer(CorsLayer::very_permissive())
}
