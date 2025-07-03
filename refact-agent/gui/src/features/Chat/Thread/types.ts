import { FThreadMessageSubs } from "../../../../generated/documents";
import { Usage } from "../../../services/refact";
import { ChatMessages } from "../../../services/refact/types";
import { parseOrElse } from "../../../utils/parseOrElse";

export type IntegrationMeta = {
  name?: string;
  path?: string;
  project?: string;
  shouldIntermediatePageShowUp?: boolean;
};

export function isIntegrationMeta(json: unknown): json is IntegrationMeta {
  if (!json || typeof json !== "object") return false;
  if (!("name" in json) || !("path" in json) || !("project" in json)) {
    return false;
  }
  return true;
}

export interface MessageWithIntegrationMeta
  extends Omit<
    FThreadMessageSubs["news_payload_thread_message"],
    "ftm_user_preferences"
  > {
  ftm_user_preferences: { integration: IntegrationMeta };
}

export function isMessageWithIntegrationMeta(
  message: unknown,
): message is MessageWithIntegrationMeta {
  if (!message || typeof message !== "object") return false;
  if (!("ftm_user_preferences" in message)) return false;
  if (
    !message.ftm_user_preferences ||
    typeof message.ftm_user_preferences !== "object"
  )
    return false;
  const preferences = message.ftm_user_preferences as Record<string, unknown>;
  if (!("integration" in preferences)) return false;
  return isIntegrationMeta(preferences.integration);
}
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
  boost_reasoning?: boolean;
  integration?: IntegrationMeta | null;
  mode?: LspChatMode;
  project_name?: string;
  last_user_message_id?: string;
  new_chat_suggested: SuggestedChat;
  automatic_patch?: boolean;
  currentMaximumContextTokens?: number;
  currentMessageContextTokens?: number;
  increase_max_tokens?: boolean;
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
  checkpoints_enabled?: boolean;
  waiting_for_response: boolean;
  max_new_tokens?: number;
  cache: Record<string, ChatThread>;
  tool_use: ToolUse;
  send_immediately: boolean;
  follow_ups_enabled?: boolean;
  title_generation_enabled?: boolean;
};

export type PayloadWithId = { id: string };
export type PayloadWithChatAndNumber = { chatId: string; value: number };
export type PayloadWithChatAndMessageId = { chatId: string; messageId: string };
export type PayloadWithChatAndBoolean = { chatId: string; value: boolean };
export type PayloadWithChatAndUsage = { chatId: string; usage: Usage };
export type PayloadWithChatAndCurrentUsage = {
  chatId: string;
  n_ctx: number;
  prompt_tokens: number;
};
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
  | "PROJECT_SUMMARY";

export function isLspChatMode(mode: string): mode is LspChatMode {
  return (
    mode === "NO_TOOLS" ||
    mode === "EXPLORE" ||
    mode === "AGENT" ||
    mode === "CONFIGURE" ||
    mode === "PROJECT_SUMMARY"
  );
}

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
    return defaultMode;
  }
  if (mode) {
    return mode;
  }
  if (toolUse === "agent") return "AGENT";
  if (toolUse === "quick") return "NO_TOOLS";
  return "EXPLORE";
}
