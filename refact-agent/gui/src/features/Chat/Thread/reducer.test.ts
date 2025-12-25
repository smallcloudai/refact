import { expect, test, describe, beforeEach } from "vitest";
import { chatReducer } from "./reducer";
import type { Chat } from "./types";
import { newChatAction, applyChatEvent } from "./actions";
import type { ChatEventEnvelope } from "../../../services/refact/chatSubscription";

describe("Chat Thread Reducer - Event-based (Stateless Trajectory UI)", () => {
  let initialState: Chat;
  let chatId: string;

  beforeEach(() => {
    const emptyState = chatReducer(undefined, { type: "@@INIT" });
    initialState = chatReducer(emptyState, newChatAction(undefined));
    chatId = initialState.current_thread_id;
  });

  describe("applyChatEvent - snapshot", () => {
    test("should initialize thread from snapshot event", () => {
      const event: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "1",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "Test Chat",
          model: "gpt-4",
          mode: "AGENT",
          tool_use: "agent",
          boost_reasoning: false,
          context_tokens_cap: 8192,
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
        messages: [
          { role: "user", content: "Hello" },
          { role: "assistant", content: "Hi there!" },
        ],
      };

      const result = chatReducer(initialState, applyChatEvent(event));
      const runtime = result.threads[chatId];

      expect(runtime).toBeDefined();
      expect(runtime?.thread.title).toBe("Test Chat");
      expect(runtime?.thread.model).toBe("gpt-4");
      expect(runtime?.thread.messages).toHaveLength(2);
      expect(runtime?.streaming).toBe(false);
      expect(runtime?.waiting_for_response).toBe(false);
    });

    test("should handle snapshot with generating state", () => {
      const event: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      const result = chatReducer(initialState, applyChatEvent(event));
      const runtime = result.threads[chatId];

      expect(runtime?.streaming).toBe(true);
      expect(runtime?.waiting_for_response).toBe(true);
    });

    test("should handle snapshot with paused state", () => {
      const event: ChatEventEnvelope = {
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
          paused: true,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      const result = chatReducer(initialState, applyChatEvent(event));
      const runtime = result.threads[chatId];

      expect(runtime?.confirmation.pause).toBe(true);
    });

    test("should handle snapshot with error state", () => {
      const event: ChatEventEnvelope = {
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
          state: "error",  // Must be "error" state for prevent_send to be true
          paused: false,
          error: "Something went wrong",
          queue_size: 0,
        },
        messages: [],
      };

      const result = chatReducer(initialState, applyChatEvent(event));
      const runtime = result.threads[chatId];

      expect(runtime?.error).toBe("Something went wrong");
      expect(runtime?.prevent_send).toBe(true);
    });
  });

  describe("applyChatEvent - stream_delta", () => {
    test("should append content via delta ops", () => {
      // First set up a thread with an assistant message that has a message_id
      const snapshotEvent: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [
          { role: "user", content: "Hello" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      // Use stream_started to add assistant message with message_id
      const streamStartEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-1",
      };

      state = chatReducer(state, applyChatEvent(streamStartEvent));

      // Now apply a delta
      const deltaEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-1",
        ops: [
          { op: "append_content", text: "Hi there!" },
        ],
      };

      state = chatReducer(state, applyChatEvent(deltaEvent));
      const runtime = state.threads[chatId];
      const lastMessage = runtime?.thread.messages[runtime.thread.messages.length - 1];

      expect(lastMessage?.content).toBe("Hi there!");
    });

    test("should handle reasoning content delta", () => {
      const snapshotEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "1",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "Test",
          model: "gpt-4",
          mode: "AGENT",
          tool_use: "agent",
          boost_reasoning: true,
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
        messages: [
          { role: "user", content: "Explain" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      // Use stream_started to add assistant message
      const streamStartEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-1",
      };

      state = chatReducer(state, applyChatEvent(streamStartEvent));

      const deltaEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-1",
        ops: [
          { op: "append_reasoning", text: "Let me think about this..." },
        ],
      };

      state = chatReducer(state, applyChatEvent(deltaEvent));
      const runtime = state.threads[chatId];
      const lastMessage = runtime?.thread.messages[runtime.thread.messages.length - 1];

      expect(lastMessage).toHaveProperty("reasoning_content", "Let me think about this...");
    });
  });

  describe("applyChatEvent - runtime_updated", () => {
    test("should update streaming state when generation starts", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const runtimeEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "runtime_updated",
        state: "generating",
        paused: false,
        error: null,
        queue_size: 0,
      };

      state = chatReducer(state, applyChatEvent(runtimeEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.streaming).toBe(true);
      expect(runtime?.waiting_for_response).toBe(true);
    });

    test("should update streaming state when generation completes", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const runtimeEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "runtime_updated",
        state: "idle",
        paused: false,
        error: null,
        queue_size: 0,
      };

      state = chatReducer(state, applyChatEvent(runtimeEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.streaming).toBe(false);
      expect(runtime?.waiting_for_response).toBe(false);
    });
  });

  describe("applyChatEvent - message_added", () => {
    test("should add message at index", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [{ role: "user", content: "Hello" }],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const addEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "message_added",
        message: { role: "assistant", content: "Hi!" },
        index: 1,
      };

      state = chatReducer(state, applyChatEvent(addEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(2);
      expect(runtime?.thread.messages[1].content).toBe("Hi!");
    });

    test("should replace existing message with same message_id (deduplication)", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [{ role: "user", content: "Hello" }],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      // First, stream_started adds a placeholder with message_id
      const streamStartEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "stream_started",
        message_id: "msg-123",
      };
      state = chatReducer(state, applyChatEvent(streamStartEvent));

      // Add some streaming content
      const deltaEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "3",
        type: "stream_delta",
        message_id: "msg-123",
        ops: [{ op: "append_content", text: "Streaming content..." }],
      };
      state = chatReducer(state, applyChatEvent(deltaEvent));

      // Now message_added comes with the same message_id - should REPLACE, not duplicate
      const addEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "4",
        type: "message_added",
        message: {
          role: "assistant",
          content: "Final complete content",
          message_id: "msg-123",
        },
        index: 1,
      };

      state = chatReducer(state, applyChatEvent(addEvent));
      const runtime = state.threads[chatId];

      // Should still have only 2 messages (user + assistant), not 3
      expect(runtime?.thread.messages).toHaveLength(2);
      // Content should be the final version, not streaming version
      expect(runtime?.thread.messages[1].content).toBe("Final complete content");
    });
  });

  describe("applyChatEvent - pause_required", () => {
    test("should set pause state and reasons", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const pauseEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "pause_required",
        reasons: [
          {
            type: "confirmation",
            command: "shell rm -rf /",
            rule: "dangerous_command",
            tool_call_id: "call_123",
            integr_config_path: null,
          },
        ],
      };

      state = chatReducer(state, applyChatEvent(pauseEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.confirmation.pause).toBe(true);
      expect(runtime?.confirmation.pause_reasons).toHaveLength(1);
      expect(runtime?.confirmation.pause_reasons[0].tool_call_id).toBe("call_123");
      // Note: streaming state is controlled by runtime_updated, not pause_required
    });
  });

  describe("applyChatEvent - runtime_updated with error", () => {
    test("should set error state via runtime_updated", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
          state: "generating",
          paused: false,
          error: null,
          queue_size: 0,
        },
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const errorEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "runtime_updated",
        state: "error",
        paused: false,
        error: "API rate limit exceeded",
        queue_size: 0,
      };

      state = chatReducer(state, applyChatEvent(errorEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.error).toBe("API rate limit exceeded");
      expect(runtime?.prevent_send).toBe(true);
      expect(runtime?.streaming).toBe(false);
    });
  });

  describe("applyChatEvent - title_updated", () => {
    test("should update thread title", () => {
      const snapshotEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "1",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "",
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
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const titleEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "title_updated",
        title: "Help with React hooks",
        is_generated: true,
      };

      state = chatReducer(state, applyChatEvent(titleEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.title).toBe("Help with React hooks");
    });
  });

  describe("applyChatEvent - message_updated", () => {
    test("should update message content by message_id", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "Original", message_id: "msg-user-1" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const updateEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "message_updated",
        message_id: "msg-user-1",
        message: { role: "user", content: "Updated content", message_id: "msg-user-1" },
      };

      state = chatReducer(state, applyChatEvent(updateEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(1);
      expect(runtime?.thread.messages[0].content).toBe("Updated content");
    });

    test("should not affect other messages when updating", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "First", message_id: "msg-1" },
          { role: "assistant", content: "Response", message_id: "msg-2" },
          { role: "user", content: "Second", message_id: "msg-3" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const updateEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "message_updated",
        message_id: "msg-2",
        message: { role: "assistant", content: "Updated response", message_id: "msg-2" },
      };

      state = chatReducer(state, applyChatEvent(updateEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(3);
      expect(runtime?.thread.messages[0].content).toBe("First");
      expect(runtime?.thread.messages[1].content).toBe("Updated response");
      expect(runtime?.thread.messages[2].content).toBe("Second");
    });
  });

  describe("applyChatEvent - message_removed", () => {
    test("should remove message by message_id", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "Hello", message_id: "msg-1" },
          { role: "assistant", content: "Hi", message_id: "msg-2" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const removeEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "message_removed",
        message_id: "msg-2",
      };

      state = chatReducer(state, applyChatEvent(removeEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(1);
      expect(runtime?.thread.messages[0].content).toBe("Hello");
    });

    test("should handle removing non-existent message gracefully", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "Hello", message_id: "msg-1" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const removeEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "message_removed",
        message_id: "non-existent-id",
      };

      state = chatReducer(state, applyChatEvent(removeEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(1);
    });
  });

  describe("applyChatEvent - messages_truncated", () => {
    test("should truncate messages from index", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "First", message_id: "msg-1" },
          { role: "assistant", content: "Response 1", message_id: "msg-2" },
          { role: "user", content: "Second", message_id: "msg-3" },
          { role: "assistant", content: "Response 2", message_id: "msg-4" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const truncateEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "messages_truncated",
        from_index: 2,
      };

      state = chatReducer(state, applyChatEvent(truncateEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(2);
      expect(runtime?.thread.messages[0].content).toBe("First");
      expect(runtime?.thread.messages[1].content).toBe("Response 1");
    });

    test("should handle truncate from index 0", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [
          { role: "user", content: "Hello", message_id: "msg-1" },
          { role: "assistant", content: "Hi", message_id: "msg-2" },
        ],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const truncateEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "messages_truncated",
        from_index: 0,
      };

      state = chatReducer(state, applyChatEvent(truncateEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.messages).toHaveLength(0);
    });
  });

  describe("applyChatEvent - thread_updated", () => {
    test("should update thread params", () => {
      const snapshotEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "1",
        type: "snapshot",
        thread: {
          id: chatId,
          title: "Test",
          model: "gpt-3.5",
          mode: "NO_TOOLS",
          tool_use: "quick",
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
        messages: [],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      const updateEvent: ChatEventEnvelope = {
        chat_id: chatId,
        seq: "2",
        type: "thread_updated",
        model: "gpt-4",
        mode: "AGENT",
        boost_reasoning: true,
      };

      state = chatReducer(state, applyChatEvent(updateEvent));
      const runtime = state.threads[chatId];

      expect(runtime?.thread.model).toBe("gpt-4");
      expect(runtime?.thread.mode).toBe("AGENT");
      expect(runtime?.thread.boost_reasoning).toBe(true);
    });
  });

  describe("Event sequence handling", () => {
    test("should ignore events for unknown chat_id", () => {
      const event: ChatEventEnvelope = {
        chat_id: "unknown-chat-id",
        seq: "1",
        type: "runtime_updated",
        state: "generating",
        paused: false,
        error: null,
        queue_size: 0,
      };

      const result = chatReducer(initialState, applyChatEvent(event));

      // State should be unchanged
      expect(result.threads["unknown-chat-id"]).toBeUndefined();
    });

    test("should process events in sequence", () => {
      const snapshotEvent: ChatEventEnvelope = {
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
        messages: [{ role: "user", content: "Hi" }],
      };

      let state = chatReducer(initialState, applyChatEvent(snapshotEvent));

      // Process sequence of events (using correct event types)
      const events: ChatEventEnvelope[] = [
        { chat_id: chatId, seq: "2", type: "runtime_updated", state: "generating", paused: false, error: null, queue_size: 0 },
        { chat_id: chatId, seq: "3", type: "stream_started", message_id: "msg-1" },
        { chat_id: chatId, seq: "4", type: "stream_delta", message_id: "msg-1", ops: [
          { op: "append_content", text: "Hello!" },
        ]},
        { chat_id: chatId, seq: "5", type: "stream_finished", message_id: "msg-1", finish_reason: "stop" },
        { chat_id: chatId, seq: "6", type: "runtime_updated", state: "idle", paused: false, error: null, queue_size: 0 },
      ];

      for (const event of events) {
        state = chatReducer(state, applyChatEvent(event));
      }

      const runtime = state.threads[chatId];
      expect(runtime?.streaming).toBe(false);
      expect(runtime?.thread.messages).toHaveLength(2);
      expect(runtime?.thread.messages[1].content).toBe("Hello!");
    });
  });
});
