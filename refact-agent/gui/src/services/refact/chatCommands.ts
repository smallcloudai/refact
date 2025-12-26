import { v4 as uuidv4 } from "uuid";

export type MessageContent =
  | string
  | (
      | { type: "text"; text: string }
      | { type: "image_url"; image_url: { url: string } }
    )[];

export type ChatCommandBase =
  | {
      type: "user_message";
      content: MessageContent;
      attachments?: unknown[];
    }
  | {
      type: "retry_from_index";
      index: number;
      content?: MessageContent;
      attachments?: unknown[];
    }
  | {
      type: "set_params";
      patch: Record<string, unknown>;
    }
  | {
      type: "abort";
    }
  | {
      type: "tool_decision";
      tool_call_id: string;
      accepted: boolean;
    }
  | {
      type: "tool_decisions";
      decisions: { tool_call_id: string; accepted: boolean }[];
    }
  | {
      type: "ide_tool_result";
      tool_call_id: string;
      content: string;
      tool_failed: boolean;
    }
  | {
      type: "update_message";
      message_id: string;
      content: MessageContent;
      attachments?: unknown[];
      regenerate?: boolean;
    }
  | {
      type: "remove_message";
      message_id: string;
      regenerate?: boolean;
    };

export type ChatCommand = ChatCommandBase & { client_request_id: string };

export async function sendChatCommand(
  chatId: string,
  port: number,
  apiKey: string | undefined,
  command: ChatCommandBase,
): Promise<void> {
  const commandWithId = {
    ...command,
    client_request_id: uuidv4(),
  };

  const url = `http://127.0.0.1:${port}/v1/chats/${encodeURIComponent(chatId)}/commands`;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (apiKey) {
    headers.Authorization = `Bearer ${apiKey}`;
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
  } as ChatCommandBase);
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
  } as ChatCommandBase);
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
  } as ChatCommandBase);
}

export async function abortGeneration(
  chatId: string,
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "abort",
  } as ChatCommandBase);
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
  } as ChatCommandBase);
}

export async function respondToToolConfirmations(
  chatId: string,
  decisions: { tool_call_id: string; accepted: boolean }[],
  port: number,
  apiKey?: string,
): Promise<void> {
  await sendChatCommand(chatId, port, apiKey, {
    type: "tool_decisions",
    decisions,
  } as ChatCommandBase);
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
  } as ChatCommandBase);
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
  } as ChatCommandBase);
}
