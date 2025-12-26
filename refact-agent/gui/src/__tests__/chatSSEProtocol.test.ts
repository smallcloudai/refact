/**
 * SSE Protocol Completeness & Correctness Tests
 *
 * Tests all ChatEvent types from backend (engine/src/chat/types.rs)
 * Validates event structure, sequence numbers, and state transitions
 *
 * Run with: npm run test:no-watch -- chatSSEProtocol
 */

/* eslint-disable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/require-await, @typescript-eslint/ban-ts-comment */
// @ts-nocheck - Testing runtime behavior with discriminated unions
import { describe, it, expect, vi, beforeEach } from "vitest";
import { subscribeToChatEvents, applyDeltaOps, type EventEnvelope, type DeltaOp } from "../services/refact/chatSubscription";
import type { ChatMessage } from "../services/refact/types";

const createMockReader = (chunks: string[]) => {
  let index = 0;
  return {
    read: vi.fn(async () => {
      if (index >= chunks.length) {
        return { done: true, value: undefined };
      }
      const encoder = new TextEncoder();
      return { done: false, value: encoder.encode(chunks[index++]) };
    }),
  };
};

const createMockFetch = (chunks: string[]) => {
  return vi.fn().mockResolvedValue({
    ok: true,
    body: {
      getReader: () => createMockReader(chunks),
    },
  });
};

describe("SSE Protocol - Event Types", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Snapshot Event", () => {
    it("should parse snapshot with all fields", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
          title: "Test Chat",
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
          pause_reasons: [],
        },
        messages: [],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([
        `data: ${JSON.stringify(snapshot)}\n\n`,
      ]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events).toHaveLength(1);
      expect(events[0].type).toBe("snapshot");
      expect(events[0].seq).toBe("0");
      expect((events[0] as any).thread.id).toBe("test-123");
      expect((events[0] as any).runtime.state).toBe("idle");
    });

    it("should handle snapshot with messages", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
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
          pause_reasons: [],
        },
        messages: [
          { role: "user", content: "Hello", message_id: "msg-1" },
          { role: "assistant", content: "Hi there", message_id: "msg-2" },
        ],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([
        `data: ${JSON.stringify(snapshot)}\n\n`,
      ]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].messages).toHaveLength(2);
      expect(events[0].messages[0].role).toBe("user");
      expect(events[0].messages[1].role).toBe("assistant");
    });
  });

  describe("Stream Events", () => {
    it("should parse stream_started event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "1",
        type: "stream_started",
        message_id: "msg-new",
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("stream_started");
      expect(events[0].message_id).toBe("msg-new");
    });

    it("should parse stream_delta with all op types", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "2",
        type: "stream_delta",
        message_id: "msg-new",
        ops: [
          { op: "append_content", text: "Hello" },
          { op: "append_reasoning", text: "thinking..." },
          { op: "set_tool_calls", tool_calls: [{ id: "call_1", function: { name: "test", arguments: "{}" } }] },
          { op: "set_thinking_blocks", blocks: [{ thinking: "step 1" }] },
          { op: "add_citation", citation: { url: "http://example.com" } },
          { op: "set_usage", usage: { prompt_tokens: 100, completion_tokens: 50 } },
          { op: "merge_extra", extra: { custom_field: "value" } },
        ],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("stream_delta");
      expect(events[0].ops).toHaveLength(7);
      expect(events[0].ops[0].op).toBe("append_content");
      expect(events[0].ops[6].op).toBe("merge_extra");
    });

    it("should parse stream_finished with all finish_reason values", async () => {
      const reasons = ["stop", "length", "abort", "error", "tool_calls", null];

      for (const reason of reasons) {
        const event: EventEnvelope = {
          chat_id: "test-123",
          seq: "3",
          type: "stream_finished",
          message_id: "msg-new",
          finish_reason: reason,
        };

        const events: EventEnvelope[] = [];
        const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
        global.fetch = mockFetch;

        subscribeToChatEvents("test-123", 8001, {
          onEvent: (e) => events.push(e),
          onError: vi.fn(),
        });

        await new Promise((resolve) => setTimeout(resolve, 10));

        expect(events[0].type).toBe("stream_finished");
        expect(events[0].finish_reason).toBe(reason);
      }
    });
  });

  describe("Message Events", () => {
    it("should parse message_added event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "4",
        type: "message_added",
        message: { role: "user", content: "New message", message_id: "msg-5" },
        index: 2,
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("message_added");
      expect(events[0].message.role).toBe("user");
      expect(events[0].index).toBe(2);
    });

    it("should parse message_updated event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "5",
        type: "message_updated",
        message_id: "msg-3",
        message: { role: "user", content: "Updated content", message_id: "msg-3" },
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("message_updated");
      expect(events[0].message_id).toBe("msg-3");
    });

    it("should parse message_removed event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "6",
        type: "message_removed",
        message_id: "msg-4",
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("message_removed");
      expect(events[0].message_id).toBe("msg-4");
    });

    it("should parse messages_truncated event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "7",
        type: "messages_truncated",
        from_index: 5,
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("messages_truncated");
      expect(events[0].from_index).toBe(5);
    });
  });

  describe("State Events", () => {
    it("should parse thread_updated event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "8",
        type: "thread_updated",
        title: "New Title",
        model: "gpt-4o",
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("thread_updated");
      expect(events[0].title).toBe("New Title");
    });

    it("should parse runtime_updated with all states", async () => {
      const states = ["idle", "generating", "executing_tools", "paused", "waiting_ide", "error"];

      for (const state of states) {
        const event: EventEnvelope = {
          chat_id: "test-123",
          seq: "9",
          type: "runtime_updated",
          state,
          paused: state === "paused",
          error: state === "error" ? "Test error" : null,
          queue_size: 0,
        };

        const events: EventEnvelope[] = [];
        const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
        global.fetch = mockFetch;

        subscribeToChatEvents("test-123", 8001, {
          onEvent: (e) => events.push(e),
          onError: vi.fn(),
        });

        await new Promise((resolve) => setTimeout(resolve, 10));

        expect(events[0].type).toBe("runtime_updated");
        expect(events[0].state).toBe(state);
      }
    });

    it("should parse title_updated event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "10",
        type: "title_updated",
        title: "Generated Title",
        is_generated: true,
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("title_updated");
      expect(events[0].title).toBe("Generated Title");
      expect(events[0].is_generated).toBe(true);
    });
  });

  describe("Pause Events", () => {
    it("should parse pause_required event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "11",
        type: "pause_required",
        reasons: [
          {
            type: "confirmation",
            command: "patch",
            rule: "always",
            tool_call_id: "call_1",
            integr_config_path: null,
          },
        ],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("pause_required");
      expect(events[0].reasons).toHaveLength(1);
      expect(events[0].reasons[0].type).toBe("confirmation");
    });

    it("should parse pause_cleared event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "12",
        type: "pause_cleared",
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("pause_cleared");
    });
  });

  describe("IDE Tool Events", () => {
    it("should parse ide_tool_required event", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "13",
        type: "ide_tool_required",
        tool_call_id: "call_ide_1",
        tool_name: "goto",
        args: { file: "test.ts", line: 42 },
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("ide_tool_required");
      expect(events[0].tool_call_id).toBe("call_ide_1");
      expect(events[0].tool_name).toBe("goto");
      expect(events[0].args).toEqual({ file: "test.ts", line: 42 });
    });
  });

  describe("Ack Events", () => {
    it("should parse ack event with success", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "14",
        type: "ack",
        client_request_id: "req-123",
        accepted: true,
        result: null,
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("ack");
      expect(events[0].client_request_id).toBe("req-123");
      expect(events[0].accepted).toBe(true);
    });

    it("should parse ack event with error", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "15",
        type: "ack",
        client_request_id: "req-456",
        accepted: false,
        result: { error: "Invalid command" },
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].type).toBe("ack");
      expect(events[0].accepted).toBe(false);
      expect(events[0].result).toEqual({ error: "Invalid command" });
    });
  });
});

describe("SSE Protocol - Sequence Numbers", () => {
  it("should accept string sequence numbers", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "42",
      type: "pause_cleared",
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].seq).toBe("42");
  });

  it("should accept numeric sequence numbers", async () => {
    const event = {
      chat_id: "test-123",
      seq: 42,
      type: "pause_cleared",
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].seq).toBe("42");
  });

  it("should handle monotonically increasing sequences", async () => {
    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([
      `data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "2", type: "pause_cleared" })}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "3", type: "pause_cleared" })}\n\n`,
    ]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(events).toHaveLength(3);
    expect(events[0].seq).toBe("1");
    expect(events[1].seq).toBe("2");
    expect(events[2].seq).toBe("3");
  });
});

describe("SSE Protocol - Field Variations", () => {
  describe("RuntimeState variations", () => {
    it("should handle runtime with pause_reasons in snapshot", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
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
          state: "paused",
          paused: true,
          error: null,
          queue_size: 1,
          pause_reasons: [
            {
              type: "confirmation",
              command: "patch",
              rule: "always",
              tool_call_id: "call_1",
              integr_config_path: null,
            },
          ],
        },
        messages: [],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].runtime.pause_reasons).toHaveLength(1);
      expect(events[0].runtime.pause_reasons[0].type).toBe("confirmation");
    });

    it("should handle runtime with error state", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
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
          state: "error",
          paused: false,
          error: "Connection timeout",
          queue_size: 0,
          pause_reasons: [],
        },
        messages: [],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].runtime.state).toBe("error");
      expect(events[0].runtime.error).toBe("Connection timeout");
    });

    it("should handle runtime with queue_size > 0", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
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
          queue_size: 5,
          pause_reasons: [],
        },
        messages: [],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].runtime.queue_size).toBe(5);
    });
  });

  describe("ThreadParams variations", () => {
    it("should handle thread with context_tokens_cap set", async () => {
      const snapshot: EventEnvelope = {
        chat_id: "test-123",
        seq: "0",
        type: "snapshot",
        thread: {
          id: "test-123",
          title: "Test",
          model: "gpt-4",
          mode: "AGENT",
          tool_use: "agent",
          boost_reasoning: true,
          context_tokens_cap: 8000,
          include_project_info: false,
          checkpoints_enabled: false,
          is_title_generated: true,
        },
        runtime: {
          state: "idle",
          paused: false,
          error: null,
          queue_size: 0,
          pause_reasons: [],
        },
        messages: [],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].thread.context_tokens_cap).toBe(8000);
      expect(events[0].thread.boost_reasoning).toBe(true);
      expect(events[0].thread.include_project_info).toBe(false);
      expect(events[0].thread.checkpoints_enabled).toBe(false);
      expect(events[0].thread.is_title_generated).toBe(true);
    });

    it("should handle thread with different modes", async () => {
      const modes = ["AGENT", "EXPLORE", "QUICK"];

      for (const mode of modes) {
        const snapshot: EventEnvelope = {
          chat_id: "test-123",
          seq: "0",
          type: "snapshot",
          thread: {
            id: "test-123",
            title: "Test",
            model: "gpt-4",
            mode,
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
            pause_reasons: [],
          },
          messages: [],
        };

        const events: EventEnvelope[] = [];
        const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
        global.fetch = mockFetch;

        subscribeToChatEvents("test-123", 8001, {
          onEvent: (e) => events.push(e),
          onError: vi.fn(),
        });

        await new Promise((resolve) => setTimeout(resolve, 10));

        expect(events[0].thread.mode).toBe(mode);
      }
    });
  });

  describe("PauseReason variations", () => {
    it("should handle pause_reason with integr_config_path", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "1",
        type: "pause_required",
        reasons: [
          {
            type: "integration",
            command: "docker_exec",
            rule: "ask",
            tool_call_id: "call_1",
            integr_config_path: "/path/to/config.yaml",
          },
        ],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].reasons[0].integr_config_path).toBe("/path/to/config.yaml");
    });

    it("should handle multiple pause_reasons", async () => {
      const event: EventEnvelope = {
        chat_id: "test-123",
        seq: "1",
        type: "pause_required",
        reasons: [
          {
            type: "confirmation",
            command: "patch",
            rule: "always",
            tool_call_id: "call_1",
            integr_config_path: null,
          },
          {
            type: "confirmation",
            command: "shell",
            rule: "ask",
            tool_call_id: "call_2",
            integr_config_path: null,
          },
        ],
      };

      const events: EventEnvelope[] = [];
      const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test-123", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].reasons).toHaveLength(2);
      expect(events[0].reasons[0].tool_call_id).toBe("call_1");
      expect(events[0].reasons[1].tool_call_id).toBe("call_2");
    });
  });
});

describe("SSE Protocol - Edge Cases", () => {
  it("should handle empty messages array in snapshot", async () => {
    const snapshot: EventEnvelope = {
      chat_id: "test-123",
      seq: "0",
      type: "snapshot",
      thread: {
        id: "test-123",
        title: "Empty",
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
        pause_reasons: [],
      },
      messages: [],
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].messages).toEqual([]);
  });

  it("should handle null finish_reason", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "1",
      type: "stream_finished",
      message_id: "msg-1",
      finish_reason: null,
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].finish_reason).toBeNull();
  });

  it("should handle empty pause_reasons array", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "1",
      type: "pause_required",
      reasons: [],
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].reasons).toEqual([]);
  });

  it("should skip [DONE] marker", async () => {
    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([
      `data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`,
      `data: [DONE]\n\n`,
    ]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
  });

  it("should handle malformed JSON gracefully", async () => {
    const events: EventEnvelope[] = [];
    const errors: Error[] = [];
    const mockFetch = createMockFetch([
      `data: {invalid json}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`,
    ]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: (e) => errors.push(e),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle messages with all ChatMessage fields", async () => {
    const snapshot: EventEnvelope = {
      chat_id: "test-123",
      seq: "0",
      type: "snapshot",
      thread: {
        id: "test-123",
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
        pause_reasons: [],
      },
      messages: [
        {
          role: "user",
          content: "Hello",
          message_id: "msg-1",
        },
        {
          role: "assistant",
          content: "Hi",
          message_id: "msg-2",
          tool_calls: [
            {
              id: "call_1",
              type: "function",
              function: { name: "test", arguments: "{}" },
            },
          ],
          finish_reason: "tool_calls",
          usage: { prompt_tokens: 10, completion_tokens: 5 },
        },
        {
          role: "tool",
          content: "Result",
          message_id: "msg-3",
          tool_call_id: "call_1",
          tool_failed: false,
        },
      ],
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].messages).toHaveLength(3);
    expect(events[0].messages[1].tool_calls).toHaveLength(1);
    expect(events[0].messages[1].finish_reason).toBe("tool_calls");
    expect(events[0].messages[2].tool_call_id).toBe("call_1");
  });

  it("should handle multimodal message content", async () => {
    const snapshot: EventEnvelope = {
      chat_id: "test-123",
      seq: "0",
      type: "snapshot",
      thread: {
        id: "test-123",
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
        pause_reasons: [],
      },
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What's in this image?" },
            { type: "image_url", image_url: { url: "data:image/png;base64,..." } },
          ],
          message_id: "msg-1",
        },
      ],
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(snapshot)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(Array.isArray(events[0].messages[0].content)).toBe(true);
    expect((events[0].messages[0].content as any)[0].type).toBe("text");
    expect((events[0].messages[0].content as any)[1].type).toBe("image_url");
  });

  it("should handle stream_delta with empty ops array", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "1",
      type: "stream_delta",
      message_id: "msg-1",
      ops: [],
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].ops).toEqual([]);
  });

  it("should handle very long sequence numbers", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "999999999999",
      type: "pause_cleared",
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].seq).toBe("999999999999");
  });

  it("should handle thread_updated with flattened params", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "1",
      type: "thread_updated",
      title: "New Title",
      model: "gpt-4o",
      boost_reasoning: true,
      custom_field: "custom_value",
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].type).toBe("thread_updated");
    expect((events[0] as any).title).toBe("New Title");
    expect((events[0] as any).custom_field).toBe("custom_value");
  });

  it("should handle ack with null result", async () => {
    const event: EventEnvelope = {
      chat_id: "test-123",
      seq: "1",
      type: "ack",
      client_request_id: "req-123",
      accepted: true,
      result: null,
    };

    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([`data: ${JSON.stringify(event)}\n\n`]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test-123", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].result).toBeNull();
  });

  it("should handle rapid event sequence", async () => {
    const events: EventEnvelope[] = [];
    const mockFetch = createMockFetch([
      `data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "stream_started", message_id: "msg-1" })}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "2", type: "stream_delta", message_id: "msg-1", ops: [{ op: "append_content", text: "H" }] })}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "3", type: "stream_delta", message_id: "msg-1", ops: [{ op: "append_content", text: "i" }] })}\n\n`,
      `data: ${JSON.stringify({ chat_id: "test", seq: "4", type: "stream_finished", message_id: "msg-1", finish_reason: "stop" })}\n\n`,
    ]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(events).toHaveLength(4);
    expect(events[0].type).toBe("stream_started");
    expect(events[1].type).toBe("stream_delta");
    expect(events[2].type).toBe("stream_delta");
    expect(events[3].type).toBe("stream_finished");
  });
});

describe("DeltaOp Application - merge_extra", () => {
  it("should merge extra fields into message.extra", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg-1",
    };

    const ops: DeltaOp[] = [
      { op: "merge_extra", extra: { metering_a: 100 } },
    ];

    const result = applyDeltaOps(message, ops) as any;
    expect(result.extra).toEqual({ metering_a: 100 });
  });

  it("should merge multiple extra fields incrementally", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg-1",
    };

    const ops: DeltaOp[] = [
      { op: "merge_extra", extra: { metering_a: 100 } },
      { op: "merge_extra", extra: { metering_b: 200 } },
      { op: "merge_extra", extra: { metering_a: 150 } },
    ];

    const result = applyDeltaOps(message, ops) as any;
    expect(result.extra).toEqual({ metering_a: 150, metering_b: 200 });
  });
});
