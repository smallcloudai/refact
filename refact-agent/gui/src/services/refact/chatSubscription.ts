import type { ChatMessage } from "./types";

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

export type RuntimeState = {
  state: "idle" | "generating" | "executing_tools" | "paused" | "waiting_ide" | "error";
  paused: boolean;
  error: string | null;
  queue_size: number;
  pause_reasons?: PauseReason[];
};

export type PauseReason = {
  type: string;
  command: string;
  rule: string;
  tool_call_id: string;
  integr_config_path: string | null;
};

export type DeltaOp =
  | { op: "append_content"; text: string }
  | { op: "append_reasoning"; text: string }
  | { op: "set_tool_calls"; tool_calls: unknown[] }
  | { op: "set_thinking_blocks"; blocks: unknown[] }
  | { op: "add_citation"; citation: unknown }
  | { op: "set_usage"; usage: unknown }
  | { op: "merge_extra"; extra: Record<string, unknown> };

export type ChatEvent =
  | {
      type: "snapshot";
      thread: ThreadParams;
      runtime: RuntimeState;
      messages: ChatMessage[];
    }
  | { type: "thread_updated" } & Partial<ThreadParams>
  | {
      type: "runtime_updated";
      state: RuntimeState["state"];
      paused: boolean;
      error: string | null;
      queue_size: number;
    }
  | { type: "title_updated"; title: string; is_generated: boolean }
  | { type: "message_added"; message: ChatMessage; index: number }
  | { type: "message_updated"; message_id: string; message: ChatMessage }
  | { type: "message_removed"; message_id: string }
  | { type: "messages_truncated"; from_index: number }
  | { type: "stream_started"; message_id: string }
  | { type: "stream_delta"; message_id: string; ops: DeltaOp[] }
  | {
      type: "stream_finished";
      message_id: string;
      finish_reason: string | null;
    }
  | { type: "pause_required"; reasons: PauseReason[] }
  | { type: "pause_cleared" }
  | {
      type: "ide_tool_required";
      tool_call_id: string;
      tool_name: string;
      args: unknown;
    }
  | {
      type: "ack";
      client_request_id: string;
      accepted: boolean;
      result?: unknown;
    };

export type ChatEventEnvelope = {
  chat_id: string;
  seq: string;
} & ChatEvent;

export type ChatSubscriptionCallbacks = {
  onEvent: (event: ChatEventEnvelope) => void;
  onError: (error: Error) => void;
  onConnected?: () => void;
  onDisconnected?: () => void;
};

function isValidChatEvent(data: unknown): data is ChatEventEnvelope {
  if (typeof data !== "object" || data === null) return false;
  const obj = data as Record<string, unknown>;
  if (typeof obj.chat_id !== "string") return false;
  if (typeof obj.seq !== "string") return false;
  if (typeof obj.type !== "string") return false;
  return true;
}

export function subscribeToChatEvents(
  chatId: string,
  port: number,
  callbacks: ChatSubscriptionCallbacks,
): () => void {
  const url = `http://127.0.0.1:${port}/v1/chats/subscribe?chat_id=${encodeURIComponent(chatId)}`;

  const eventSource = new EventSource(url);

  eventSource.onopen = () => {
    callbacks.onConnected?.();
  };

  eventSource.onmessage = (event) => {
    try {
      const parsed = JSON.parse(event.data) as unknown;
      if (!isValidChatEvent(parsed)) {
        console.error("Invalid chat event structure:", parsed);
        return;
      }
      callbacks.onEvent(parsed);
    } catch (e) {
      console.error("Failed to parse chat event:", e, event.data);
    }
  };

  eventSource.onerror = () => {
    callbacks.onError(new Error("SSE connection error"));
    if (eventSource.readyState === EventSource.CLOSED) {
      callbacks.onDisconnected?.();
    }
  };

  return () => {
    eventSource.close();
    callbacks.onDisconnected?.();
  };
}

export function applyDeltaOps(
  message: ChatMessage,
  ops: DeltaOp[],
): ChatMessage {
  // Create a shallow copy - we'll mutate this
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updated: any = { ...message };

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
          (updated.reasoning_content || "") + op.text;
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
        Object.assign(updated, op.extra);
        break;
    }
  }

  return updated as ChatMessage;
}
