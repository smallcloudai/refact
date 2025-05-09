use axum::Router;
use axum::routing::{get, post, delete};
use tower_http::cors::CorsLayer;

use crate::http::utils::telemetry_middleware;
use crate::http::routers::v1::code_completion::{handle_v1_code_completion_web, handle_v1_code_completion_prompt};
use crate::http::routers::v1::code_lens::handle_v1_code_lens;
use crate::http::routers::v1::ast::{handle_v1_ast_file_dump, handle_v1_ast_file_symbols, handle_v1_ast_status};
use crate::http::routers::v1::at_commands::{handle_v1_command_completion, handle_v1_command_preview, handle_v1_at_command_execute};
use crate::http::routers::v1::at_tools::{handle_v1_tools, handle_v1_tools_check_if_confirmation_needed, handle_v1_tools_execute};
use crate::http::routers::v1::caps::handle_v1_caps;
use crate::http::routers::v1::caps::handle_v1_ping;
use crate::http::routers::v1::chat::{handle_v1_chat, handle_v1_chat_completions};
use crate::http::routers::v1::chat_based_handlers::{handle_v1_commit_message_from_diff, handle_v1_trajectory_compress};
use crate::http::routers::v1::chat_based_handlers::handle_v1_trajectory_save;
use crate::http::routers::v1::dashboard::get_dashboard_plots;
use crate::http::routers::v1::docker::{handle_v1_docker_container_action, handle_v1_docker_container_list};
use crate::http::routers::v1::git::{handle_v1_git_commit, handle_v1_checkpoints_preview, handle_v1_checkpoints_restore};
use crate::http::routers::v1::graceful_shutdown::handle_v1_graceful_shutdown;
use crate::http::routers::v1::mcp::{handle_mcp_servers, handle_mcp_resources};
use crate::http::routers::v1::snippet_accepted::handle_v1_snippet_accepted;
use crate::http::routers::v1::telemetry_network::handle_v1_telemetry_network;
use crate::http::routers::v1::telemetry_chat::handle_v1_telemetry_chat;
use crate::http::routers::v1::links::handle_v1_links;
use crate::http::routers::v1::lsp_like_handlers::{handle_v1_lsp_did_change, handle_v1_lsp_add_folder, handle_v1_lsp_initialize, handle_v1_lsp_remove_folder, handle_v1_set_active_document};
use crate::http::routers::v1::status::handle_v1_rag_status;
use crate::http::routers::v1::customization::handle_v1_customization;
use crate::http::routers::v1::customization::handle_v1_config_path;
use crate::http::routers::v1::gui_help_handlers::handle_v1_fullpath;
use crate::http::routers::v1::subchat::{handle_v1_subchat, handle_v1_subchat_single};
use crate::http::routers::v1::sync_files::handle_v1_sync_files_extract_tar;
use crate::http::routers::v1::system_prompt::handle_v1_prepend_system_prompt_and_maybe_more_initial_messages;
use crate::http::routers::v1::providers::{handle_v1_providers, handle_v1_provider_templates,
    handle_v1_get_model, handle_v1_get_provider, handle_v1_models, handle_v1_post_model, handle_v1_post_provider,
    handle_v1_delete_model, handle_v1_delete_provider, handle_v1_model_default, handle_v1_completion_model_families};

#[cfg(feature = "vecdb")]
use crate::http::routers::v1::vecdb::{handle_v1_vecdb_search, handle_v1_vecdb_status};
#[cfg(feature="vecdb")]
use crate::http::routers::v1::handlers_memdb::{handle_mem_add, handle_mem_erase, handle_mem_update_used, handle_mem_block_until_vectorized};
use crate::http::routers::v1::v1_integrations::{handle_v1_integration_get, handle_v1_integration_icon, handle_v1_integration_save, handle_v1_integration_delete, handle_v1_integrations, handle_v1_integrations_filtered, handle_v1_integrations_mcp_logs};
use crate::agent_db::db_cthread::{handle_db_v1_cthread_update, handle_db_v1_cthreads_sub};
use crate::agent_db::db_cmessage::{handle_db_v1_cmessages_update, handle_db_v1_cmessages_sub};
use crate::agent_db::db_chore::{handle_db_v1_chore_update, handle_db_v1_chore_event_update, handle_db_v1_chores_sub};
use crate::http::routers::v1::file_edit_tools::handle_v1_file_edit_tool_dry_run;
use crate::http::routers::v1::handlers_memdb::{handle_mem_sub, handle_mem_upd};

mod ast;
pub mod at_commands;
pub mod at_tools;
pub mod caps;
pub mod chat;
pub mod chat_based_handlers;
pub mod code_completion;
pub mod code_lens;
pub mod customization;
mod dashboard;
mod mcp;
mod docker;
mod git;
pub mod graceful_shutdown;
mod gui_help_handlers;
pub mod links;
pub mod lsp_like_handlers;
pub mod snippet_accepted;
pub mod status;
mod subchat;
pub mod sync_files;
pub mod system_prompt;
pub mod telemetry_chat;
pub mod telemetry_network;
pub mod providers;

mod file_edit_tools;
#[cfg(feature = "vecdb")]
pub mod handlers_memdb;
mod v1_integrations;
#[cfg(feature = "vecdb")]
pub mod vecdb;

pub fn make_v1_router() -> Router {
    let builder = Router::new()
        .route("/ping", get(handle_v1_ping))
        .route("/graceful-shutdown", get(handle_v1_graceful_shutdown))

        .route("/code-completion", post(handle_v1_code_completion_web))
        .route("/code-lens", post(handle_v1_code_lens))

        .route("/chat", post(handle_v1_chat))
        .route("/chat/completions", post(handle_v1_chat_completions))  // standard

        .route("/telemetry-network", post(handle_v1_telemetry_network))
        .route("/telemetry-chat", post(handle_v1_telemetry_chat))
        .route("/snippet-accepted", post(handle_v1_snippet_accepted))

        .route("/caps", get(handle_v1_caps))

        .route("/tools", get(handle_v1_tools))
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

        .route("/sync-files-extract-tar", post(handle_v1_sync_files_extract_tar))

        .route("/git-commit", post(handle_v1_git_commit))

        .route("/prepend-system-prompt-and-maybe-more-initial-messages",
            post(handle_v1_prepend_system_prompt_and_maybe_more_initial_messages)) // because it works remotely

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
        .route("/mcp-servers", get(handle_mcp_servers))
        .route("/mcp-resources", get(handle_mcp_resources))

        .route("/docker-container-list", post(handle_v1_docker_container_list))
        .route("/docker-container-action", post(handle_v1_docker_container_action))

        .route("/checkpoints-preview", post(handle_v1_checkpoints_preview))
        .route("/checkpoints-restore", post(handle_v1_checkpoints_restore))

        .route("/links", post(handle_v1_links))

        .route("/file_edit_tool_dry_run", post(handle_v1_file_edit_tool_dry_run))

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

        // experimental
        .route("/get-dashboard-plots", get(get_dashboard_plots))

        .route("/code-completion-prompt", post(handle_v1_code_completion_prompt))
        .route("/commit-message-from-diff", post(handle_v1_commit_message_from_diff))

        // to remove
        .route("/subchat", post(handle_v1_subchat))
        .route("/subchat-single", post(handle_v1_subchat_single))
        ;

    #[cfg(feature = "vecdb")]
    let builder = builder
        .route("/vdb-search", post(handle_v1_vecdb_search))
        .route("/vdb-status", get(handle_v1_vecdb_status))
        .route("/mem-add", post(handle_mem_add))
        .route("/mem-erase", post(handle_mem_erase))
        .route("/mem-upd", post(handle_mem_upd))
        .route("/mem-update-used", post(handle_mem_update_used))
        .route("/mem-block-until-vectorized", get(handle_mem_block_until_vectorized))
        .route("/mem-sub", post(handle_mem_sub))
        .route("/trajectory-save", post(handle_v1_trajectory_save))
        .route("/trajectory-compress", post(handle_v1_trajectory_compress))
        ;

    builder
        .layer(axum::middleware::from_fn(telemetry_middleware))
        .layer(CorsLayer::very_permissive())
}

pub fn make_db_v1_router() -> Router {
    let builder = Router::new()
        .route("/cthreads-sub", post(handle_db_v1_cthreads_sub))
        .route("/cthread-update", post(handle_db_v1_cthread_update))
        .route("/cmessages-sub", post(handle_db_v1_cmessages_sub))
        .route("/cmessages-update", post(handle_db_v1_cmessages_update))
        .route("/chores-sub", post(handle_db_v1_chores_sub))
        .route("/chore-update", post(handle_db_v1_chore_update))
        .route("/chore-event-update", post(handle_db_v1_chore_event_update));
    builder
        .layer(axum::middleware::from_fn(telemetry_middleware))
        .layer(CorsLayer::very_permissive())
}
