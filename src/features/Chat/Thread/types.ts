import { SystemPrompts } from "../../../services/refact/prompts";
import { ChatMessages } from "../../../services/refact/types";
import { parseOrElse } from "../../../utils/parseOrElse";

export type ChatThread = {
  id: string;
  messages: ChatMessages;
  model: string;
  title?: string;
  createdAt?: string;
  updatedAt?: string;
  read?: boolean;
};

export type ToolUse = "quick" | "explore" | "agent";

export type Chat = {
  streaming: boolean;
  thread: ChatThread;
  error: null | string;
  prevent_send: boolean;
  waiting_for_response: boolean;
  cache: Record<string, ChatThread>;
  system_prompt: SystemPrompts;
  tool_use: ToolUse;
  send_immediately: boolean;
};

export type PayloadWithId = { id: string };
export type PayloadWithIdAndTitle = { title: string } & PayloadWithId;

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
