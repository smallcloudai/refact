import { expect, test, describe, beforeEach } from "vitest";
import { chatReducer } from "./reducer";
import type { Chat } from "./types";
import { newChatAction, applyChatEvent } from "./actions";
import type { ChatEventEnvelope } from "../../../services/refact/chatSubscription";

describe("Chat Thread Reducer - Edge Cases", () => {
  let initialState: Chat;
  let chatId: string;

  beforeEach(() => {
    const emptyState = chatReducer(undefined, { type: "@@INIT" });
    initialState = chatReducer(emptyState, newChatAction(undefined));
    chatId = initialState.current_thread_id;
  });

  const createSnapshot = (messages: unknown[] = []): ChatEventEnvelope => ({
    chat_id: chatId,
    seq: "1",
    type: "snapshot",
    thread: {
      id: chatId,
      title: "Test",
      model: "gpt-4",
      mode: "AGENT",
      tool_use: "agent",
      boost_reasoning: false,
      context_tokens_cap: null,
      include_project_info: true,
      checkpoints_enabled: true,
      is_title_generated: false,
    },
    runtime: {
      state: "idle",
      paused: false,
      error: null,
      queue_size: 0,
    },
    messages,
  });

  describe("preserve streaming fields on final message_added", () => {
    test("should keep reasoning_content from streaming when message_added arrives", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-123",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      const deltaWithReasoning: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-123",
        ops: [
          { op: "append_reasoning", text: "Let me think about this..." },
          { op: "append_content", text: "Here is my answer" },
        ],
      };
      state = chatReducer(state, applyChatEvent(deltaWithReasoning));

      const messageAdded: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "message_added",
        message: {
          message_id: "msg-123",
          role: "assistant",
          content: "Here is my answer",
        },
        index: 1,
      };
      state = chatReducer(state, applyChatEvent(messageAdded));

      const runtime = state.threads[chatId];
      const assistantMsg = runtime?.thread.messages[1];

      expect(assistantMsg?.content).toBe("Here is my answer");
      expect(assistantMsg?.reasoning_content).toBe("Let me think about this...");
    });

    test("should keep thinking_blocks from streaming when message_added arrives", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-456",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      const deltaWithThinking: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-456",
        ops: [
          { op: "set_thinking_blocks", blocks: [{ type: "thinking", thinking: "Deep thought" }] },
          { op: "append_content", text: "Answer" },
        ],
      };
      state = chatReducer(state, applyChatEvent(deltaWithThinking));

      const messageAdded: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "message_added",
        message: {
          message_id: "msg-456",
          role: "assistant",
          content: "Answer",
        },
        index: 1,
      };
      state = chatReducer(state, applyChatEvent(messageAdded));

      const runtime = state.threads[chatId];
      const assistantMsg = runtime?.thread.messages[1];

      expect(assistantMsg?.thinking_blocks).toBeDefined();
      expect(assistantMsg?.thinking_blocks?.length).toBe(1);
    });

    test("should keep usage from streaming when message_added arrives", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-789",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      const deltaWithUsage: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-789",
        ops: [
          { op: "append_content", text: "Response" },
          { op: "set_usage", usage: { prompt_tokens: 100, completion_tokens: 50, total_tokens: 150 } },
        ],
      };
      state = chatReducer(state, applyChatEvent(deltaWithUsage));

      const messageAdded: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "message_added",
        message: {
          message_id: "msg-789",
          role: "assistant",
          content: "Response",
        },
        index: 1,
      };
      state = chatReducer(state, applyChatEvent(messageAdded));

      const runtime = state.threads[chatId];
      const assistantMsg = runtime?.thread.messages[1];

      expect(assistantMsg?.usage).toBeDefined();
      expect(assistantMsg?.usage?.prompt_tokens).toBe(100);
    });
  });

  describe("empty snapshot does not wipe messages", () => {
    test("should preserve messages when snapshot has empty messages array", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
        { role: "assistant", content: "Hi there!" },
      ])));

      const runtime1 = state.threads[chatId];
      expect(runtime1?.thread.messages).toHaveLength(2);

      const emptySnapshot: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "Test",
          model: "gpt-4",
          mode: "AGENT",
          tool_use: "agent",
          boost_reasoning: false,
          context_tokens_cap: null,
          include_project_info: true,
          checkpoints_enabled: true,
          is_title_generated: false,
        },
        runtime: {
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      state = chatReducer(state, applyChatEvent(emptySnapshot));
      const runtime2 = state.threads[chatId];

      expect(runtime2?.thread.messages).toHaveLength(2);
      expect(runtime2?.thread.messages[0].content).toBe("Hello");
    });

    test("should preserve thread state when empty snapshot arrives (lag recovery)", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const emptySnapshot: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "Updated Title",
          model: "gpt-4o",
          mode: "EXPLORE",
          tool_use: "explore",
          boost_reasoning: true,
          context_tokens_cap: 4096,
          include_project_info: false,
          checkpoints_enabled: false,
          is_title_generated: true,
        },
        runtime: {
          state: "generating",
          paused: false,
          error: null,
          queue_size: 1,
        },
        messages: [],
      };

      state = chatReducer(state, applyChatEvent(emptySnapshot));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(1);
      expect(runtime?.thread.messages[0].content).toBe("Hello");
    });
  });

  describe("merge_extra safety", () => {
    test("should merge extra fields incrementally", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-extra",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      const delta1: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-extra",
        ops: [
          { op: "merge_extra", extra: { metering_a: 100 } },
        ],
      };
      state = chatReducer(state, applyChatEvent(delta1));

      const delta2: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "stream_delta",
        message_id: "msg-extra",
        ops: [
          { op: "merge_extra", extra: { metering_b: 200 } },
        ],
      };
      state = chatReducer(state, applyChatEvent(delta2));

      const delta3: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "5",
        type: "stream_delta",
        message_id: "msg-extra",
        ops: [
          { op: "merge_extra", extra: { metering_a: 150 } },
        ],
      };
      state = chatReducer(state, applyChatEvent(delta3));

      const runtime = state.threads[chatId];
      const msg = runtime?.thread.messages.find(m => m.message_id === "msg-extra") as Record<string, unknown> | undefined;

      expect(msg?.metering_a).toBe(150);
      expect(msg?.metering_b).toBe(200);
    });
  });

  describe("abort event sequence", () => {
    test("should handle stream_finished with abort reason", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-abort",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      expect(state.threads[chatId]?.streaming).toBe(true);

      const streamFinished: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_finished",
        message_id: "msg-abort",
        finish_reason: "abort",
      };
      state = chatReducer(state, applyChatEvent(streamFinished));

      const messageRemoved: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "message_removed",
        message_id: "msg-abort",
      };
      state = chatReducer(state, applyChatEvent(messageRemoved));

      const runtimeIdle: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "5",
        type: "runtime_updated",
        state: "idle",
        paused: false,
        error: null,
        queue_size: 0,
      };
      state = chatReducer(state, applyChatEvent(runtimeIdle));

      const runtime = state.threads[chatId];
      expect(runtime?.streaming).toBe(false);
      expect(runtime?.thread.messages).toHaveLength(1);
      expect(runtime?.thread.messages[0].role).toBe("user");
    });
  });

  describe("pause lifecycle events", () => {
    test("should handle pause_required event", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Run shell command" },
      ])));

      const pauseRequired: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "pause_required",
        reasons: [
          {
            type: "confirmation",
            command: "shell",
            rule: "deny_all",
            tool_call_id: "tc-1",
            integr_config_path: null,
          },
        ],
      };
      state = chatReducer(state, applyChatEvent(pauseRequired));

      const runtime = state.threads[chatId];
      expect(runtime?.confirmation.pause).toBe(true);
      expect(runtime?.confirmation.pause_reasons).toHaveLength(1);
    });

    test("should handle pause_cleared event", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([])));

      const pauseRequired: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "pause_required",
        reasons: [{ type: "confirmation", command: "shell", rule: "deny_all", tool_call_id: "tc-1", integr_config_path: null }],
      };
      state = chatReducer(state, applyChatEvent(pauseRequired));
      expect(state.threads[chatId]?.confirmation.pause).toBe(true);

      const pauseCleared: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "pause_cleared",
      };
      state = chatReducer(state, applyChatEvent(pauseCleared));

      expect(state.threads[chatId]?.confirmation.pause).toBe(false);
      expect(state.threads[chatId]?.confirmation.pause_reasons).toHaveLength(0);
    });
  });

  describe("error state handling", () => {
    test("should handle error without content (message_removed path)", () => {
      let state = chatReducer(initialState, applyChatEvent(createSnapshot([
        { role: "user", content: "Hello" },
      ])));

      const streamStart: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-error",
      };
      state = chatReducer(state, applyChatEvent(streamStart));

      const messageRemoved: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "message_removed",
        message_id: "msg-error",
      };
      state = chatReducer(state, applyChatEvent(messageRemoved));

      const errorState: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "runtime_updated",
        state: "error",
        paused: false,
        error: "Model not found",
        queue_size: 0,
      };
      state = chatReducer(state, applyChatEvent(errorState));

      const runtime = state.threads[chatId];
      expect(runtime?.error).toBe("Model not found");
      expect(runtime?.thread.messages).toHaveLength(1);
      expect(runtime?.streaming).toBe(false);
    });
  });
});
