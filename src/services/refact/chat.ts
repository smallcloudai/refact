import { getApiKey } from "../../utils/ApiKey";
import { CHAT_URL } from "./consts";
import {
  type ChatRole,
  type ChatMessages,
  type ToolCall,
  isAssistantMessage,
  isToolMessage,
} from "./types";
import { getAvailableTools } from "./tools";

type StreamArgs =
  | {
      stream: true;
      abortController: AbortController;
    }
  | { stream: false; abortController?: undefined | AbortController };

type SendChatArgs = {
  messages: LspChatMessage[];
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
} & StreamArgs;

export type LspChatMessage = {
  role: ChatRole;
  content: string | null;
  tool_calls?: Omit<ToolCall, "index">[];
  tool_call_id?: string;
};

export function formatMessagesForLsp(messages: ChatMessages): LspChatMessage[] {
  return messages.reduce<LspChatMessage[]>((acc, message) => {
    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message[0],
          content: message[1],
          tool_calls: message[2] ?? undefined,
        },
      ]);
    }

    if (isToolMessage(message)) {
      return acc.concat([
        {
          role: "tool",
          content: message[1].content,
          tool_call_id: message[1].tool_call_id,
        },
      ]);
    }

    const content =
      typeof message[1] === "string" ? message[1] : JSON.stringify(message[1]);
    return [...acc, { role: message[0], content }];
  }, []);
}

export async function sendChat({
  messages,
  model,
  abortController,
  stream,
  lspUrl,
  takeNote = false,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
}: SendChatArgs): Promise<Response> {
  const toolsResponse = await getAvailableTools();

  const tools = takeNote
    ? toolsResponse.filter(
        (tool) => tool.function.name === "remember_how_to_use_tools",
      )
    : toolsResponse.filter(
        (tool) => tool.function.name !== "remember_how_to_use_tools",
      );

  const body = JSON.stringify({
    messages,
    model: model,
    parameters: {
      max_new_tokens: 2048,
    },
    stream,
    tools: tools,
    max_tokens: 2048,
    only_deterministic_messages,
    chat_id,
  });

  const apiKey = getApiKey();
  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  };
  const chatEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${CHAT_URL}`
    : CHAT_URL;

  return fetch(chatEndpoint, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    signal: abortController?.signal,
    credentials: "same-origin",
  });
}
