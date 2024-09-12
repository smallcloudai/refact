import { CHAT_URL } from "./consts";
import { ToolCommand } from "./tools";
import { ChatRole, ToolCall } from "./types";

export type LspChatMessage = {
  role: ChatRole;
  content: string | null;
  tool_calls?: Omit<ToolCall, "index">[];
  tool_call_id?: string;
};

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
    parameters: {
      max_new_tokens: 2048,
    },
    stream,
    tools,
    max_tokens: 2048,
    only_deterministic_messages,
    chat_id,
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
  // lspUrl,
  // takeNote = false,
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
    credentials: "same-origin",
  });
}
