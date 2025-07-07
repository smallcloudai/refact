import { ToolConfirmationRequest } from "../features/ThreadMessages/threadMessagesSlice";

export const CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH: ToolConfirmationRequest[] =
  [
    {
      command: "SELECT *",
      rule: "*",
      ftm_num: 2,
      // type: "confirmation",
      tool_call_id: "1",
      // integr_config_path: "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
    },
  ];
export const CONFIRMATIONAL_PAUSE_REASONS: ToolConfirmationRequest[] = [
  {
    command: "patch",
    rule: "default",
    ftm_num: 2,
    // type: "confirmation",
    tool_call_id: "1",
    // integr_config_path: null,
  },
];

export const DENIAL_PAUSE_REASONS_WITH_PATH: ToolConfirmationRequest[] = [
  {
    command: "SELECT *",
    rule: "*",
    ftm_num: 2,
    // type: "denial",
    tool_call_id: "1",
    // integr_config_path:
    //   "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
];

export const MIXED_PAUSE_REASONS: ToolConfirmationRequest[] = [
  {
    command: "SELECT *",
    rule: "*",
    // type: "denial",
    ftm_num: 2,
    tool_call_id: "1",
    // integr_config_path:
    //   "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
  {
    command: "DROP *",
    rule: "*",
    // type: "confirmation",
    tool_call_id: "1",
    ftm_num: 2,
    // integr_config_path:
    //   "\\\\?\\d:\\work\\refact.ai\\refact-lsp\\.refact\\integrations\\postgres.yaml",
  },
];
