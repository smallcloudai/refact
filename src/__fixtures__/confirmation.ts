import { ToolConfirmationPauseReason } from "../services/refact";

export const CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH: ToolConfirmationPauseReason[] =
  [
    {
      command: "SELECT *",
      rule: "*",
      type: "confirmation",
      tool_call_id: "1",
      integr_config_path:
        "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
    },
  ];
export const CONFIRMATIONAL_PAUSE_REASONS: ToolConfirmationPauseReason[] = [
  {
    command: "patch",
    rule: "default",
    type: "confirmation",
    tool_call_id: "1",
    integr_config_path: null,
  },
];

export const DENIAL_PAUSE_REASONS_WITH_PATH: ToolConfirmationPauseReason[] = [
  {
    command: "SELECT *",
    rule: "*",
    type: "denial",
    tool_call_id: "1",
    integr_config_path:
      "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
];

export const MIXED_PAUSE_REASONS: ToolConfirmationPauseReason[] = [
  {
    command: "SELECT *",
    rule: "*",
    type: "denial",
    tool_call_id: "1",
    integr_config_path:
      "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
  {
    command: "DROP *",
    rule: "*",
    type: "confirmation",
    tool_call_id: "1",
    integr_config_path:
      "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
];
