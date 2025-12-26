import type { ChatMessage } from "./types";

export type SessionState = 
  | "idle" 
  | "generating" 
  | "executing_tools" 
  | "paused" 
  | "waiting_ide" 
  | "error";

export type ThreadParams = {
  id: string;
  title: string;
  model: string;
  mode: string;
  tool_use: string;
  boost_reasoning: boolean;
  context_tokens_cap: number | null;
  include_project_info: boolean;
  checkpoints_enabled: boolean;
  is_title_generated: boolean;
};

export type PauseReason = {
  type: string;
  command: string;
  rule: string;
  tool_call_id: string;
  integr_config_path: string | null;
};

export type RuntimeState = {
  state: SessionState;
  paused: boolean;
  error: string | null;
  queue_size: number;
  pause_reasons: PauseReason[];
};

export type DeltaOp =
  | { op: "append_content"; text: string }
  | { op: "append_reasoning"; text: string }
  | { op: "set_tool_calls"; tool_calls: unknown[] }
  | { op: "set_thinking_blocks"; blocks: unknown[] }
  | { op: "add_citation"; citation: unknown }
  | { op: "set_usage"; usage: unknown }
  | { op: "merge_extra"; extra: Record<string, unknown> };

export type EventEnvelope = 
  | {
      chat_id: string;
      seq: string;
      type: "snapshot";
      thread: ThreadParams;
      runtime: RuntimeState;
      messages: ChatMessage[];
    }
  | {
      chat_id: string;
      seq: string;
      type: "thread_updated";
      [key: string]: unknown;
    }
  | {
      chat_id: string;
      seq: string;
      type: "runtime_updated";
      state: SessionState;
      paused: boolean;
      error: string | null;
      queue_size: number;
    }
  | {
      chat_id: string;
      seq: string;
      type: "title_updated";
      title: string;
      is_generated: boolean;
    }
  | {
      chat_id: string;
      seq: string;
      type: "message_added";
      message: ChatMessage;
      index: number;
    }
  | {
      chat_id: string;
      seq: string;
      type: "message_updated";
      message_id: string;
      message: ChatMessage;
    }
  | {
      chat_id: string;
      seq: string;
      type: "message_removed";
      message_id: string;
    }
  | {
      chat_id: string;
      seq: string;
      type: "messages_truncated";
      from_index: number;
    }
  | {
      chat_id: string;
      seq: string;
      type: "stream_started";
      message_id: string;
    }
  | {
      chat_id: string;
      seq: string;
      type: "stream_delta";
      message_id: string;
      ops: DeltaOp[];
    }
  | {
      chat_id: string;
      seq: string;
      type: "stream_finished";
      message_id: string;
      finish_reason: string | null;
    }
  | {
      chat_id: string;
      seq: string;
      type: "pause_required";
      reasons: PauseReason[];
    }
  | {
      chat_id: string;
      seq: string;
      type: "pause_cleared";
    }
  | {
      chat_id: string;
      seq: string;
      type: "ide_tool_required";
      tool_call_id: string;
      tool_name: string;
      args: unknown;
    }
  | {
      chat_id: string;
      seq: string;
      type: "ack";
      client_request_id: string;
      accepted: boolean;
      result: unknown;
    };

export type ChatEventEnvelope = EventEnvelope;

export type ChatEventType = EventEnvelope["type"];

export type ChatSubscriptionCallbacks = {
  onEvent: (event: EventEnvelope) => void;
  onError: (error: Error) => void;
  onConnected?: () => void;
  onDisconnected?: () => void;
};

export type SubscriptionOptions = Record<string, never>;

export function subscribeToChatEvents(
  chatId: string,
  port: number,
  callbacks: ChatSubscriptionCallbacks,
  apiKey?: string,
): () => void {
  const url = `http://127.0.0.1:${port}/v1/chats/subscribe?chat_id=${encodeURIComponent(chatId)}`;

  const abortController = new AbortController();
  const state = { connected: false };

  const headers: Record<string, string> = {};
  if (apiKey) {
    headers.Authorization = `Bearer ${apiKey}`;
  }

  const disconnect = () => {
    if (state.connected) {
      state.connected = false;
      callbacks.onDisconnected?.();
    }
  };

  void fetch(url, {
    method: "GET",
    headers,
    signal: abortController.signal,
  })
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`SSE connection failed: ${response.status}`);
      }

      if (!response.body) {
        throw new Error("Response body is null");
      }

      state.connected = true;
      callbacks.onConnected?.();

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        buffer = buffer.replace(/\r\n/g, "\n").replace(/\r/g, "\n");

        const blocks = buffer.split("\n\n");
        buffer = blocks.pop() ?? "";

        for (const block of blocks) {
          if (!block.trim()) continue;

          const dataLines: string[] = [];
          for (const rawLine of block.split("\n")) {
            if (!rawLine.startsWith("data:")) continue;
            dataLines.push(rawLine.slice(5).replace(/^\s*/, ""));
          }

          if (dataLines.length === 0) continue;

          const dataStr = dataLines.join("\n");
          if (dataStr === "[DONE]") continue;

          try {
            const parsed = JSON.parse(dataStr) as unknown;
            if (!isValidChatEventBasic(parsed)) {
              continue;
            }
            normalizeSeq(parsed);
            callbacks.onEvent(parsed);
          } catch {
            // Parse error, skip this event
          }
        }
      }

      disconnect();
    })
    .catch((err: unknown) => {
      const error = err as Error;
      if (error.name !== "AbortError") {
        callbacks.onError(error);
        disconnect();
      }
    });

  return () => {
    abortController.abort();
    disconnect();
  };
}

function isValidChatEventBasic(data: unknown): data is EventEnvelope {
  if (typeof data !== "object" || data === null) return false;
  const obj = data as Record<string, unknown>;
  if (typeof obj.chat_id !== "string") return false;
  if (typeof obj.seq !== "string" && typeof obj.seq !== "number") return false;
  if (typeof obj.type !== "string") return false;
  return true;
}

function normalizeSeq(obj: EventEnvelope): void {
  const s = obj.seq as string | number;
  if (typeof s === "string") {
    const trimmed = s.trim();
    if (!/^\d+$/.test(trimmed)) {
      throw new Error("Invalid seq string");
    }
    (obj as { seq: string }).seq = trimmed;
    return;
  }
  if (typeof s === "number") {
    if (!Number.isFinite(s) || !Number.isInteger(s) || s < 0) {
      throw new Error("Invalid seq number");
    }
    (obj as { seq: string }).seq = String(s);
    return;
  }
  throw new Error("Missing/invalid seq");
}

export function applyDeltaOps(
  message: ChatMessage,
  ops: DeltaOp[],
): ChatMessage {
  const updated = { ...message } as ChatMessage & {
    content?: string;
    reasoning_content?: string;
    tool_calls?: unknown[];
    thinking_blocks?: unknown[];
    citations?: unknown[];
    usage?: unknown;
    extra?: Record<string, unknown>;
  };

  for (const op of ops) {
    switch (op.op) {
      case "append_content":
        if (typeof updated.content === "string") {
          updated.content = updated.content + op.text;
        } else {
          updated.content = op.text;
        }
        break;

      case "append_reasoning":
        updated.reasoning_content =
          (updated.reasoning_content ?? "") + op.text;
        break;

      case "set_tool_calls":
        updated.tool_calls = op.tool_calls;
        break;

      case "set_thinking_blocks":
        updated.thinking_blocks = op.blocks;
        break;

      case "add_citation":
        if (!updated.citations) {
          updated.citations = [];
        }
        updated.citations.push(op.citation);
        break;

      case "set_usage":
        updated.usage = op.usage;
        break;

      case "merge_extra":
        updated.extra = { ...(updated.extra ?? {}), ...op.extra };
        break;
    }
  }

  return updated;
}
