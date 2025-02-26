import { IntegrationMeta, LspChatMode } from "../../features/Chat";
import { CHAT_URL } from "./consts";
import { ToolCommand } from "./tools";
import { ChatRole, ToolCall, ToolResult, UserMessage } from "./types";
import { CallEngineConfig, getServerUrl } from "./call_engine";

export const DEFAULT_MAX_NEW_TOKENS = 4096;
export const INCREASED_MAX_NEW_TOKENS = 16384;

export type LspChatMessage =
  | {
      role: ChatRole;
      content: string | null;
      finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
      tool_calls?: ToolCall[];
      tool_call_id?: string;
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

type StreamArgs =
  | {
      stream: true;
      abortSignal: AbortSignal;
    }
  | { stream: false; abortSignal?: undefined | AbortSignal };

type SendChatArgs = {
  messages: LspChatMessage[];
  last_user_message_id?: string;
  model: string;
  max_new_tokens?: number;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  tools: ToolCommand[] | null;
  port?: number;
  apiKey?: string | null;
  toolsConfirmed?: boolean;
  checkpointsEnabled?: boolean;
  integration?: IntegrationMeta | null;
  mode?: LspChatMode;
} & StreamArgs;

type GetChatTitleArgs = {
  messages: LspChatMessage[];
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  port?: number;
  apiKey?: string | null;
} & StreamArgs;

export type GetChatTitleResponse = {
  choices: Choice[];
  created: number;
  deterministic_messages: DeterministicMessage[];
  id: string;
  metering_balance: number;
  model: string;
  object: string;
  system_fingerprint: string;
  usage: Usage;
};

export type GetChatTitleActionPayload = {
  chatId: string;
  title: string;
};

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

type CompletionTokenDetails = {
  accepted_prediction_tokens: number;
  audio_tokens: number;
  reasoning_tokens: number;
  rejected_prediction_tokens: number;
};

type PromptTokenDetails = {
  audio_tokens: number;
  cached_tokens: number;
};

export type Usage = {
  completion_tokens: number;
  prompt_tokens: number;
  total_tokens: number;
  completion_tokens_details: CompletionTokenDetails | null;
  prompt_tokens_details: PromptTokenDetails | null;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
};

export async function sendChat({
  messages,
  model,
  abortSignal,
  stream,
  max_new_tokens,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  tools,
  port = 8001,
  apiKey,
  toolsConfirmed = true,
  checkpointsEnabled = true,
  integration,
  last_user_message_id = "",
  mode,
  lspUrl = "",
}: SendChatArgs): Promise<Response> {
  const body = JSON.stringify({
    messages,
    model: model,
    stream,
    tools,
    max_tokens: max_new_tokens,
    only_deterministic_messages,
    tools_confirmation: toolsConfirmed,
    checkpoints_enabled: checkpointsEnabled,
    meta: {
      chat_id,
      request_attempt_id: last_user_message_id,
      chat_mode: mode ?? "EXPLORE",
      ...(integration?.path ? { current_config_file: integration.path } : {}),
    },
  });

  const config: CallEngineConfig = {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
    body,
    signal: abortSignal,
    redirect: "follow",
    cache: "no-cache",
    credentials: "same-origin",
  };

  const url = getServerUrl({ config: { lspPort: port, lspUrl } } as RootState, CHAT_URL);
  return fetch(url, config);
}

export async function generateChatTitle({
  messages,
  model,
  stream,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  port = 8001,
  apiKey,
  lspUrl,
}: GetChatTitleArgs): Promise<Response> {
  const body = JSON.stringify({
    messages,
    model: model,
    stream,
    max_tokens: 300,
    only_deterministic_messages,
    chat_id,
  });

  const config: CallEngineConfig = {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
    body,
    redirect: "follow",
    cache: "no-cache",
    credentials: "same-origin",
  };

  const url = getServerUrl({ config: { lspPort: port, lspUrl } } as RootState, CHAT_URL);
  return fetch(url, config);
}