import { IntegrationMeta, LspChatMode } from "../../features/Chat";
import { CHAT_URL } from "./consts";
import { ToolCommand } from "./tools";
import { ChatRole, ToolCall, ToolResult, UserMessage } from "./types";

export type LspChatMessage =
  | {
      role: ChatRole;
      // TODO make this a union type for user message
      content: string | null;
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

type StreamArgs =
  | {
      stream: true;
      abortSignal: AbortSignal;
    }
  | { stream: false; abortSignal?: undefined | AbortSignal };

type SendChatArgs = {
  messages: LspChatMessage[];
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  tools: ToolCommand[] | null;
  port?: number;
  apiKey?: string | null;
  // isConfig?: boolean;
  toolsConfirmed?: boolean;
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

export type Usage = {
  completion_tokens: number;
  prompt_tokens: number;
  total_tokens: number;
};
// TODO: add config url
export async function sendChat({
  messages,
  model,
  abortSignal,
  stream,
  // lspUrl,
  // takeNote = false,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  tools,
  port = 8001,
  apiKey,
  toolsConfirmed = true,
  // isConfig = false,
  integration,
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
    max_tokens: 4096,
    only_deterministic_messages,
    tools_confirmation: toolsConfirmed,
    // chat_id,
    meta: {
      chat_id,
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
    parameters: {
      max_new_tokens: 2048,
    },
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
