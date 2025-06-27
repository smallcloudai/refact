use at_tools::handle_v1_post_tools;
use axum::Router;
use axum::routing::{get, post, delete};
use tower_http::cors::CorsLayer;

use crate::http::routers::v1::code_completion::{handle_v1_code_completion_web, handle_v1_code_completion_prompt};
use crate::http::routers::v1::code_lens::handle_v1_code_lens;
use crate::http::routers::v1::ast::{handle_v1_ast_file_dump, handle_v1_ast_file_symbols, handle_v1_ast_status};
use crate::http::routers::v1::at_commands::{handle_v1_command_completion, handle_v1_command_preview, handle_v1_at_command_execute};
use crate::http::routers::v1::at_tools::{handle_v1_get_tools, handle_v1_tools_check_if_confirmation_needed, handle_v1_tools_execute};
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::caps::handle_v1_ping;
use crate::http::routers::v1::chat_based_handlers::{handle_v1_commit_message_from_diff, handle_v1_trajectory_compress};
use crate::http::routers::v1::chat_based_handlers::handle_v1_trajectory_save;
use crate::http::routers::v1::docker::{handle_v1_docker_container_action, handle_v1_docker_container_list};
use crate::http::routers::v1::git::{handle_v1_git_commit, handle_v1_checkpoints_preview, handle_v1_checkpoints_restore};
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::links::handle_v1_links;
use crate::http::routers::v1::lsp_like_handlers::{handle_v1_lsp_did_change, handle_v1_lsp_add_folder, handle_v1_lsp_initialize, handle_v1_lsp_remove_folder, handle_v1_set_active_document};
use crate::http::routers::v1::status::handle_v1_rag_status;
use crate::http::routers::v1::customization::handle_v1_customization;
use crate::http::routers::v1::customization::handle_v1_config_path;
use crate::http::routers::v1::gui_help_handlers::handle_v1_fullpath;
use crate::http::routers::v1::providers::{handle_v1_providers, handle_v1_provider_templates,
    handle_v1_get_model, handle_v1_get_provider, handle_v1_models, handle_v1_post_model, handle_v1_post_provider,
    handle_v1_delete_model, handle_v1_delete_provider, handle_v1_model_default, handle_v1_completion_model_families};

use crate::http::routers::v1::vecdb::{handle_v1_vecdb_search, handle_v1_vecdb_status};
use crate::http::routers::v1::v1_integrations::{handle_v1_integration_get, handle_v1_integration_icon, handle_v1_integration_save, handle_v1_integration_delete, handle_v1_integrations, handle_v1_integrations_filtered, handle_v1_integrations_mcp_logs};
use crate::http::routers::v1::file_edit_tools::handle_v1_file_edit_tool_dry_run;
use crate::http::routers::v1::workspace::{handle_v1_get_app_searchable_id, handle_v1_set_active_group_id};

mod ast;
pub mod at_commands;
pub mod at_tools;
pub mod caps;
pub mod chat_based_handlers;
pub mod code_completion;
pub mod code_lens;
pub mod customization;
mod docker;
mod git;
pub mod graceful_shutdown;
mod gui_help_handlers;
pub mod links;
pub mod lsp_like_handlers;
pub mod status;
pub mod providers;
mod file_edit_tools;
mod v1_integrations;
pub mod vecdb;
mod workspace;

pub fn make_v1_router() -> Router {
    let builder = Router::new()
        .route("/ping", get(handle_v1_ping))
        .route("/graceful-shutdown", get(handle_v1_graceful_shutdown))

        .route("/code-completion", post(handle_v1_code_completion_web))
        .route("/code-lens", post(handle_v1_code_lens))

        .route("/caps", get(handle_v1_caps))

        .route("/tools", get(handle_v1_get_tools))
        .route("/tools", post(handle_v1_post_tools))
        .route("/tools-check-if-confirmation-needed", post(handle_v1_tools_check_if_confirmation_needed))
        .route("/tools-execute", post(handle_v1_tools_execute)) // because it works remotely

        .route("/lsp-initialize", post(handle_v1_lsp_initialize))
        .route("/lsp-did-changed", post(handle_v1_lsp_did_change))
        .route("/lsp-add-folder", post(handle_v1_lsp_add_folder))
        .route("/lsp-remove-folder", post(handle_v1_lsp_remove_folder))
        .route("/lsp-set-active-document", post(handle_v1_set_active_document))

        .route("/ast-file-symbols", post(handle_v1_ast_file_symbols))
        .route("/ast-file-dump", post(handle_v1_ast_file_dump))
        .route("/ast-status", get(handle_v1_ast_status))

        .route("/rag-status", get(handle_v1_rag_status))
        .route("/config-path", get(handle_v1_config_path))

        .route("/customization", get(handle_v1_customization))
        .route("/git-commit", post(handle_v1_git_commit))

        .route("/at-command-completion", post(handle_v1_command_completion))
        .route("/at-command-preview", post(handle_v1_command_preview))
        .route("/at-command-execute", post(handle_v1_at_command_execute)) // because it works remotely

        .route("/fullpath", post(handle_v1_fullpath))

        .route("/integrations", get(handle_v1_integrations))
        .route("/integrations-filtered/:integr_name", get(handle_v1_integrations_filtered))
        .route("/integration-get", post(handle_v1_integration_get))
        .route("/integration-save", post(handle_v1_integration_save))
        .route("/integration-delete", delete(handle_v1_integration_delete))
        .route("/integration-icon/:icon_name", get(handle_v1_integration_icon))
        .route("/integrations-mcp-logs", post(handle_v1_integrations_mcp_logs))

        .route("/docker-container-list", post(handle_v1_docker_container_list))
        .route("/docker-container-action", post(handle_v1_docker_container_action))

        .route("/checkpoints-preview", post(handle_v1_checkpoints_preview))
        .route("/checkpoints-restore", post(handle_v1_checkpoints_restore))

        .route("/links", post(handle_v1_links))

        .route("/file_edit_tool_dry_run", post(handle_v1_file_edit_tool_dry_run))
        
        // TODO: outdated, remove them all except completion models
        .route("/providers", get(handle_v1_providers))
        .route("/provider-templates", get(handle_v1_provider_templates))
        .route("/provider", get(handle_v1_get_provider))
        .route("/provider", post(handle_v1_post_provider))
        .route("/provider", delete(handle_v1_delete_provider))
        .route("/models", get(handle_v1_models))
        .route("/model", get(handle_v1_get_model))
        .route("/model", post(handle_v1_post_model))
        .route("/model", delete(handle_v1_delete_model))
        .route("/model-defaults", get(handle_v1_model_default))
        .route("/completion-model-families", get(handle_v1_completion_model_families))

        // cloud related 
        .route("/set-active-group-id", post(handle_v1_set_active_group_id))
        .route("/get-app-searchable-id", get(handle_v1_get_app_searchable_id))

        .route("/code-completion-prompt", post(handle_v1_code_completion_prompt))
        .route("/commit-message-from-diff", post(handle_v1_commit_message_from_diff))

        // vecdb
        .route("/vdb-search", post(handle_v1_vecdb_search))
        .route("/vdb-status", get(handle_v1_vecdb_status))
        .route("/trajectory-save", post(handle_v1_trajectory_save))
        .route("/trajectory-compress", post(handle_v1_trajectory_compress))
        ;
    builder.layer(CorsLayer::very_permissive())
}
