export const CHAT_URL = `/v1/chat`;
export const CAPS_URL = `/v1/caps`;
export const STATISTIC_URL = `/v1/get-dashboard-plots`;
export const AT_COMMAND_COMPLETION = "/v1/at-command-completion";
export const AT_COMMAND_PREVIEW = "/v1/at-command-preview";
export const CUSTOM_PROMPTS_URL = "/v1/customization";
export const TOOLS = "/v1/tools";
export const TOOLS_CHECK_CONFIRMATION =
  "/v1/tools-check-if-confirmation-needed";
export const EDIT_TOOL_DRY_RUN_URL = "/v1/file_edit_tool_dry_run";
export const CONFIG_PATH_URL = "/v1/config-path";
export const FULL_PATH_URL = "/v1/fullpath";
// TODO: add a service for the docs.
export const DOCUMENTATION_LIST = `/v1/docs-list`;
export const DOCUMENTATION_ADD = `/v1/docs-add`;
export const DOCUMENTATION_REMOVE = `/v1/docs-remove`;
export const PING_URL = `/v1/ping`;
export const PATCH_URL = `/v1/patch-single-file-from-ticket`;
export const APPLY_ALL_URL = "/v1/patch-apply-all";
export const CHAT_LINKS_URL = "/v1/links";
export const CHAT_COMMIT_LINK_URL = "/v1/git-commit";
// Integrations
export const INTEGRATIONS_URL = "/v1/integrations";
export const INTEGRATION_GET_URL = "/v1/integration-get";
export const INTEGRATION_MCP_LOGS_PATH = "/v1/integrations-mcp-logs";
export const INTEGRATION_SAVE_URL = "/v1/integration-save";
export const INTEGRATION_DELETE_URL = "/v1/integration-delete";
// Docker endpoints
export const DOCKER_CONTAINER_LIST = "/v1/docker-container-list";
export const DOCKER_CONTAINER_ACTION = "/v1/docker-container-action";
// Agent rollback endpoints
export const PREVIEW_CHECKPOINTS = "/v1/checkpoints-preview";
export const RESTORE_CHECKPOINTS = "/v1/checkpoints-restore";

export const TELEMETRY_CHAT_PATH = "/v1/telemetry-chat";
export const TELEMETRY_NET_PATH = "/v1/telemetry-network";

export const KNOWLEDGE_SUB_URL = "/v1/mem-sub";
export const KNOWLEDGE_ADD_URL = "/v1/mem-add";
export const KNOWLEDGE_REMOVE_URL = "/v1/mem-erase";
export const KNOWLEDGE_UPDATE_USED_URL = "/v1/mem-update-used";
export const KNOWLEDGE_UPDATE_URL = "/v1/mem-upd";
export const KNOWLEDGE_CREATE_URL = "/v1/trajectory-save";

export const COMPRESS_MESSAGES_URL = "/v1/trajectory-compress";

// Providers & Models
export const CONFIGURED_PROVIDERS_URL = "/v1/providers";
export const PROVIDER_TEMPLATES_URL = "/v1/provider-templates";
export const PROVIDER_URL = "/v1/provider";

export const MODELS_URL = "/v1/models";
export const MODEL_URL = "/v1/model";
export const MODEL_DEFAULTS_URL = "/v1/model-defaults";
export const COMPLETION_MODEL_FAMILIES_URL = "/v1/completion-model-families";
