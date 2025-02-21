import { IntegrationMeta, LspChatMode } from "../../features/Chat";
import { CHAT_URL } from "./consts";
import { ToolCommand } from "./tools";
import { ChatRole, ToolCall, ToolResult, UserMessage } from "./types";

export const DEFAULT_MAX_NEW_TOKENS = 4096;
export const INCREASED_MAX_NEW_TOKENS = 16384;

export type LspChatMessage =
  | {
      role: ChatRole;
      // TODO make this a union type for user message
      content: string | null;
      finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
      // TBD: why was index omitted ?
      // tool_calls?: Omit<ToolCall, "index">[];
      tool_calls?: ToolCall[];
      tool_call_id?: string;
    }
  | UserMessage
  | { role: "tool"; content: ToolResult["content"]; tool_call_id: string };

// could be more narrow.
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
  last_user_message_id?: string; // used for `refact-message-id` header
  model: string;
  max_new_tokens?: number;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  tools: ToolCommand[] | null;
  port?: number;
  apiKey?: string | null;
  // isConfig?: boolean;
  toolsConfirmed?: boolean;
  checkpointsEnabled?: boolean;
  integration?: IntegrationMeta | null;
  mode?: LspChatMode; // used for chat actions
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

// TODO: add config url
export async function sendChat({
  messages,
  model,
  abortSignal,
  stream,
  max_new_tokens,
  // lspUrl,
  // takeNote = false,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  tools,
  port = 8001,
  apiKey,
  toolsConfirmed = true,
  checkpointsEnabled = true,
  // isConfig = false,
  integration,
  last_user_message_id = "",
  mode,
}: SendChatArgs): Promise<Response> {
  // const toolsResponse = await getAvailableTools();

  // const tools = takeNote
  //   ? toolsResponse.filter(
  //       (tool) => tool.function.name === "remember_how_to_use_tools",
  //     )
  //   : toolsResponse.filter(
  //       (tool) => tool.function.name !== "remember_how_to_use_tools",
  //     );

  const body = JSON.stringify({
    messages,
    model: model,
    stream,
    tools,
    max_tokens: max_new_tokens,
    only_deterministic_messages,
    tools_confirmation: toolsConfirmed,
    checkpoints_enabled: checkpointsEnabled,
    // chat_id,
    meta: {
      chat_id,
      request_attempt_id: last_user_message_id,
      // chat_remote,
      // TODO: pass this through
      chat_mode: mode ?? "EXPLORE",
      // chat_mode: "EXPLORE", // NOTOOLS, EXPLORE, AGENT, CONFIGURE, PROJECTSUMMARY,
      // TODO: not clear, that if we set integration.path it's going to be set also in meta as current_config_file
      ...(integration?.path ? { current_config_file: integration.path } : {}),
    },
  });

  //   const apiKey = getApiKey();
  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  };

  const url = `http://127.0.0.1:${port}${CHAT_URL}`;

  return fetch(url, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    // TODO: causes an error during tests :/
    // referrer: "no-referrer",
    signal: abortSignal,
    credentials: "same-origin",
  });
}

export async function generateChatTitle({
  messages,
  model,
  stream,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  port = 8001,
  apiKey,
}: GetChatTitleArgs): Promise<Response> {
  const body = JSON.stringify({
    messages,
    model: model,
    stream,
    max_tokens: 300,
    only_deterministic_messages,
    chat_id,
  });

  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  };

  const url = `http://127.0.0.1:${port}${CHAT_URL}`;

  return fetch(url, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    // TODO: causes an error during tests :/
    // referrer: "no-referrer",
    credentials: "same-origin",
  });
}
