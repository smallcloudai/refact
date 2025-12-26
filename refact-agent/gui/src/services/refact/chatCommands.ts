import { v4 as uuidv4 } from "uuid";

export type MessageContent =
  | string
  | Array<
      | { type: "text"; text: string }
      | { type: "image_url"; image_url: { url: string } }
    >;

export type ChatCommand =
  | {
      type: "user_message";
      content: MessageContent;
      attachments?: unknown[];
      client_request_id: string;
    }
  | {
      type: "retry_from_index";
      index: number;
      content?: MessageContent;
      attachments?: unknown[];
      client_request_id: string;
    }
  | {
      type: "set_params";
      patch: Record<string, unknown>;
      client_request_id: string;
    }
  | {
      type: "abort";
      client_request_id: string;
    }
  | {
      type: "tool_decision";
      tool_call_id: string;
      accepted: boolean;
      client_request_id: string;
    }
  | {
      type: "tool_decisions";
      decisions: Array<{ tool_call_id: string; accepted: boolean }>;
      client_request_id: string;
    }
  | {
      type: "ide_tool_result";
      tool_call_id: string;
      content: string;
      tool_failed: boolean;
      client_request_id: string;
    }
  | {
      type: "update_message";
      message_id: string;
      content: MessageContent;
      attachments?: unknown[];
      regenerate?: boolean;
      client_request_id: string;
    }
  | {
      type: "remove_message";
      message_id: string;
      regenerate?: boolean;
      client_request_id: string;
    };

export async function sendChatCommand(
  chatId: string,
  port: number,
  apiKey: string | undefined,
  command: Omit<ChatCommand, "client_request_id">,
): Promise<void> {
  const commandWithId: ChatCommand = {
    ...command,
    client_request_id: uuidv4(),
  } as ChatCommand;

  const url = `http://127.0.0.1:${port}/v1/chats/${encodeURIComponent(chatId)}/commands`;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (apiKey) {
    headers["Authorization"] = `Bearer ${apiKey}`;
  }

  const response = await fetch(url, {
    method: "POST",
    headers,
    body: JSON.stringify(commandWithId),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(
      `Failed to send command: ${response.status} ${response.statusText} - ${text}`,
    );
  }
}

export async function sendUserMessage(
  chatId: string,
  content: MessageContent,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "user_message",
    content,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function retryFromIndex(
  chatId: string,
  index: number,
  content: MessageContent,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "retry_from_index",
    index,
    content,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function updateChatParams(
  chatId: string,
  params: Record<string, unknown>,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "set_params",
    patch: params,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function abortGeneration(
  chatId: string,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "abort",
  } as Omit<ChatCommand, "client_request_id">);
}

export async function respondToToolConfirmation(
  chatId: string,
  toolCallId: string,
  accepted: boolean,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "tool_decision",
    tool_call_id: toolCallId,
    accepted,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function respondToToolConfirmations(
  chatId: string,
  decisions: Array<{ tool_call_id: string; accepted: boolean }>,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "tool_decisions",
    decisions,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function updateMessage(
  chatId: string,
  messageId: string,
  content: MessageContent,
  port: number,
  apiKey?: string,
  regenerate?: boolean,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "update_message",
    message_id: messageId,
    content,
    regenerate,
  } as Omit<ChatCommand, "client_request_id">);
}

export async function removeMessage(
  chatId: string,
  messageId: string,
  port: number,
  apiKey?: string,
  regenerate?: boolean,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "remove_message",
    message_id: messageId,
    regenerate,
  } as Omit<ChatCommand, "client_request_id">);
}
