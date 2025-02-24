import { Usage } from "../../../services/refact";
import { SystemPrompts } from "../../../services/refact/prompts";
import { ChatMessages } from "../../../services/refact/types";
import { parseOrElse } from "../../../utils/parseOrElse";

export type IntegrationMeta = {
  name?: string;
  path?: string;
  project?: string;
  shouldIntermediatePageShowUp?: boolean;
};
export type ChatThread = {
  id: string;
  messages: ChatMessages;
  model: string;
  title?: string;
  createdAt?: string;
  updatedAt?: string;
  tool_use?: ToolUse;
  read?: boolean;
  isTitleGenerated?: boolean;
  integration?: IntegrationMeta | null;
  mode?: LspChatMode;
  project_name?: string;
  last_user_message_id?: string;
  new_chat_suggested: SuggestedChat;
  usage?: Usage;
  currentMaximumContextTokens?: number;
};

export type SuggestedChat = {
  wasSuggested: boolean;
  wasRejectedByUser?: boolean;
};

export type ToolUse = "quick" | "explore" | "agent";

export type Chat = {
  streaming: boolean;
  thread: ChatThread;
  error: null | string;
  prevent_send: boolean;
  automatic_patch?: boolean;
  checkpoints_enabled?: boolean;
  waiting_for_response: boolean;
  max_new_tokens?: number;
  cache: Record<string, ChatThread>;
  system_prompt: SystemPrompts;
  tool_use: ToolUse;
  send_immediately: boolean;
};

export type PayloadWithId = { id: string };
export type PayloadWithChatAndNumber = { chatId: string; value: number };
export type PayloadWithChatAndMessageId = { chatId: string; messageId: string };
export type PayloadWithChatAndBoolean = { chatId: string; value: boolean };
export type PayloadWithChatAndUsage = { chatId: string; usage: Usage };
export type PayloadWithIdAndTitle = {
  title: string;
  isTitleGenerated: boolean;
} & PayloadWithId;

export type DetailMessage = { detail: string };

function isDetailMessage(json: unknown): json is DetailMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  return "detail" in json && typeof json.detail === "string";
}

export function checkForDetailMessage(str: string): DetailMessage | false {
  const json = parseOrElse(str, {});
  if (isDetailMessage(json)) return json;
  return false;
}

export function isToolUse(str: string): str is ToolUse {
  if (!str) return false;
  if (typeof str !== "string") return false;
  return str === "quick" || str === "explore" || str === "agent";
}

export type LspChatMode =
  | "NO_TOOLS"
  | "EXPLORE"
  | "AGENT"
  | "CONFIGURE"
  | "PROJECT_SUMMARY"
  | "THINKING_AGENT";

export function chatModeToLspMode({
  toolUse,
  mode,
  defaultMode,
}: {
  toolUse?: ToolUse;
  mode?: LspChatMode;
  defaultMode?: LspChatMode;
}): LspChatMode {
  if (defaultMode) {
    if (defaultMode === "AGENT" || defaultMode === "THINKING_AGENT")
      return "AGENT";
    return defaultMode;
  }
  if (mode) {
    return mode;
  }
  if (toolUse === "agent") return "AGENT";
  if (toolUse === "quick") return "NO_TOOLS";
  return "EXPLORE";
}
