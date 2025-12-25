/**
 * Chat Commands Service
 *
 * REST API for sending commands to the engine.
 * Commands are queued and processed by the engine,
 * results come back via the SSE subscription.
 */

import type { ThreadParams } from "./chatSubscription";

// Content can be simple text or multi-modal
export type MessageContent =
  | string
  | Array<
      | { type: "text"; text: string }
      | { type: "image_url"; image_url: { url: string } }
    >;

// All command types
export type ChatCommand =
  | {
      type: "user_message";
      content: MessageContent;
      attachments?: unknown[];
    }
  | {
      type: "retry_from_index";
      index: number;
      content: MessageContent;
      attachments?: unknown[];
    }
  | {
      type: "set_params";
      patch: Partial<ThreadParams>;
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
      decisions: Array<{ tool_call_id: string; accepted: boolean }>;
    }
  | {
      type: "ide_tool_result";
      tool_call_id: string;
      content: string;
      tool_failed?: boolean;
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

// Command request with client-generated ID for deduplication
export type CommandRequest = {
  client_request_id: string;
} & ChatCommand;

// Response from command endpoint
export type CommandResponse = {
  status: "accepted" | "duplicate";
};

/**
 * Generate a unique client request ID.
 */
function generateRequestId(): string {
  return crypto.randomUUID();
}

/**
 * Send a command to the chat engine.
 *
 * @param chatId - Target chat ID
 * @param command - Command to send
 * @param port - LSP server port (default 8001)
 * @returns Command response
 */
export async function sendChatCommand(
  chatId: string,
  command: ChatCommand,
  port: number,
): Promise<CommandResponse> {
  const url = `http://127.0.0.1:${port}/v1/chats/${encodeURIComponent(chatId)}/commands`;

  const request: CommandRequest = {
    client_request_id: generateRequestId(),
    ...command,
  };

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Command failed: ${response.status} ${text}`);
  }

  return response.json() as Promise<CommandResponse>;
}

// Convenience functions for common commands

/**
 * Send a user message to the chat.
 */
export function sendUserMessage(
  chatId: string,
  content: MessageContent,
  port: number,
  attachments?: unknown[],
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "user_message",
      content,
      attachments,
    },
    port,
  );
}

/**
 * Retry from a specific message index.
 * Truncates all messages from the given index and sends a new user message.
 */
export function retryFromIndex(
  chatId: string,
  index: number,
  content: MessageContent,
  port: number,
  attachments?: unknown[],
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "retry_from_index",
      index,
      content,
      attachments,
    },
    port,
  );
}

/**
 * Update chat parameters (model, mode, etc.).
 */
export function updateChatParams(
  chatId: string,
  patch: Partial<ThreadParams>,
  port: number,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "set_params",
      patch,
    },
    port,
  );
}

/**
 * Abort the current generation.
 */
export function abortGeneration(
  chatId: string,
  port: number,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "abort",
    },
    port,
  );
}

/**
 * Accept or reject a tool call that needs confirmation.
 */
export function respondToToolConfirmation(
  chatId: string,
  toolCallId: string,
  accepted: boolean,
  port: number,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "tool_decision",
      tool_call_id: toolCallId,
      accepted,
    },
    port,
  );
}

/**
 * Accept or reject multiple tool calls at once (batch).
 */
export function respondToToolConfirmations(
  chatId: string,
  decisions: Array<{ tool_call_id: string; accepted: boolean }>,
  port: number,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "tool_decisions",
      decisions,
    },
    port,
  );
}

/**
 * Send IDE tool result back to the engine.
 */
export function sendIdeToolResult(
  chatId: string,
  toolCallId: string,
  content: string,
  port: number,
  toolFailed = false,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "ide_tool_result",
      tool_call_id: toolCallId,
      content,
      tool_failed: toolFailed,
    },
    port,
  );
}

/**
 * Update an existing message content.
 */
export function updateMessage(
  chatId: string,
  messageId: string,
  content: MessageContent,
  port: number,
  regenerate = false,
  attachments?: unknown[],
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "update_message",
      message_id: messageId,
      content,
      attachments,
      regenerate,
    },
    port,
  );
}

/**
 * Remove a message from the thread.
 */
export function removeMessage(
  chatId: string,
  messageId: string,
  port: number,
  regenerate = false,
): Promise<CommandResponse> {
  return sendChatCommand(
    chatId,
    {
      type: "remove_message",
      message_id: messageId,
      regenerate,
    },
    port,
  );
}
