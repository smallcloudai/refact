import { ChatRole, ThinkingBlock, ToolCall, ToolResult, UserMessage } from "./types";

export const DEFAULT_MAX_NEW_TOKENS = null;

export type LspChatMessage =
  | {
      role: ChatRole;
      content: string | null;
      finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
      thinking_blocks?: ThinkingBlock[];
      tool_calls?: ToolCall[];
      tool_call_id?: string;
      usage?: Usage | null;
    }
  | UserMessage
  | { role: "tool"; content: ToolResult["content"]; tool_call_id: string };

export function isLspChatMessage(json: unknown): json is LspChatMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  if (typeof json.role !== "string") return false;
  if (!("content" in json)) return false;
  if (json.content !== null && typeof json.content !== "string") return false;
  return true;
}

export function isLspUserMessage(
  message: LspChatMessage,
): message is UserMessage {
  return message.role === "user";
}

export type Choice = {
  finish_reason: string;
  index: number;
  message: Message;
};

export type Message = {
  content: string;
  role: string;
};

export type DeterministicMessage = {
  content: string;
  role: string;
  tool_call_id: string;
  usage: unknown;
};

export type CompletionTokenDetails = {
  accepted_prediction_tokens: number | null;
  audio_tokens: number | null;
  reasoning_tokens: number | null;
  rejected_prediction_tokens: number | null;
};

export type PromptTokenDetails = {
  audio_tokens: number | null;
  cached_tokens: number;
};

export type Usage = {
  completion_tokens: number;
  prompt_tokens: number;
  total_tokens: number;
  completion_tokens_details?: CompletionTokenDetails | null;
  prompt_tokens_details?: PromptTokenDetails | null;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
};


