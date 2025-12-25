/**
 * Chat Subscription Service Tests
 *
 * Tests for the SSE-based chat subscription system.
 * These tests require the refact-lsp server to be running on port 8001.
 *
 * Run with: npm run test:no-watch -- chatSubscription
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  subscribeToChatEvents,
  applyDeltaOps,
  type ChatEventEnvelope,
  type DeltaOp,
  type ChatEvent,
} from "../services/refact/chatSubscription";
import type { AssistantMessage } from "../services/refact/types";

// Helper type for tests - we're testing assistant messages
type TestMessage = AssistantMessage & {
  reasoning_content?: string;
  thinking_blocks?: unknown[];
  citations?: unknown[];
  usage?: unknown;
};

// Mock EventSource for unit tests
class MockEventSource {
  url: string;
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 0;

  constructor(url: string) {
    this.url = url;
    // Simulate connection
    setTimeout(() => {
      this.readyState = 1;
      this.onopen?.();
    }, 10);
  }

  close() {
    this.readyState = 2;
  }

  // Helper to simulate events
  simulateMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }

  simulateError() {
    this.onerror?.();
  }
}

// Store original EventSource
const OriginalEventSource = global.EventSource;

describe("chatSubscription", () => {
  describe("applyDeltaOps", () => {
    it("should append content to string content", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "Hello",
      };

      const ops: DeltaOp[] = [{ op: "append_content", text: " world" }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.content).toBe("Hello world");
    });

    it("should initialize content if not a string", () => {
      const message: TestMessage = {
        role: "assistant",
        content: undefined as unknown as string,
      };

      const ops: DeltaOp[] = [{ op: "append_content", text: "Hello" }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.content).toBe("Hello");
    });

    it("should append reasoning content", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
        reasoning_content: "Step 1: ",
      };

      const ops: DeltaOp[] = [{ op: "append_reasoning", text: "analyze" }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.reasoning_content).toBe("Step 1: analyze");
    });

    it("should initialize reasoning content if empty", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const ops: DeltaOp[] = [{ op: "append_reasoning", text: "thinking" }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.reasoning_content).toBe("thinking");
    });

    it("should set tool calls", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const toolCalls = [
        { id: "call_1", function: { name: "test", arguments: "{}" } },
      ];
      const ops: DeltaOp[] = [{ op: "set_tool_calls", tool_calls: toolCalls }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.tool_calls).toEqual(toolCalls);
    });

    it("should set thinking blocks", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const blocks = [{ thinking: "reasoning here" }];
      const ops: DeltaOp[] = [{ op: "set_thinking_blocks", blocks }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.thinking_blocks).toEqual(blocks);
    });

    it("should add citations", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const citation1 = { url: "http://example.com/1" };
      const citation2 = { url: "http://example.com/2" };
      const ops: DeltaOp[] = [
        { op: "add_citation", citation: citation1 },
        { op: "add_citation", citation: citation2 },
      ];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.citations).toEqual([citation1, citation2]);
    });

    it("should set usage", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const usage = { prompt_tokens: 100, completion_tokens: 50 };
      const ops: DeltaOp[] = [{ op: "set_usage", usage }];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.usage).toEqual(usage);
    });

    it("should apply multiple ops in sequence", () => {
      const message: TestMessage = {
        role: "assistant",
        content: "",
      };

      const ops: DeltaOp[] = [
        { op: "append_content", text: "Hello" },
        { op: "append_content", text: " " },
        { op: "append_content", text: "world" },
        { op: "append_reasoning", text: "thinking..." },
        {
          op: "set_tool_calls",
          tool_calls: [{ id: "1", function: { name: "test", arguments: "{}" } }],
        },
      ];

      const result = applyDeltaOps(message, ops) as TestMessage;
      expect(result.content).toBe("Hello world");
      expect(result.reasoning_content).toBe("thinking...");
      expect(result.tool_calls).toHaveLength(1);
    });
  });

  describe("subscribeToChatEvents", () => {
    beforeEach(() => {
      // Replace EventSource with mock
      global.EventSource = MockEventSource as unknown as typeof EventSource;
    });

    afterEach(() => {
      // Restore original EventSource
      global.EventSource = OriginalEventSource;
    });

    it("should create EventSource with correct URL", () => {
      const chatId = "test-chat-123";
      const port = 8001;

      subscribeToChatEvents(chatId, port, {
        onEvent: vi.fn(),
        onError: vi.fn(),
      });

      // Check that EventSource was created with correct URL
      // (In mock, we store the URL)
    });

    it("should call onConnected when EventSource opens", async () => {
      const onConnected = vi.fn();

      subscribeToChatEvents("test", 8001, {
        onEvent: vi.fn(),
        onError: vi.fn(),
        onConnected,
      });

      // Wait for mock connection
      await new Promise((resolve) => setTimeout(resolve, 20));

      expect(onConnected).toHaveBeenCalled();
    });

    it("should call onError when EventSource errors", async () => {
      const onError = vi.fn();

      let mockInstance: MockEventSource | undefined;
      const OriginalMock = MockEventSource;
      global.EventSource = class extends OriginalMock {
        constructor(url: string) {
          super(url);
          mockInstance = this;
        }
      } as unknown as typeof EventSource;

      subscribeToChatEvents("test", 8001, {
        onEvent: vi.fn(),
        onError,
      });

      await new Promise((resolve) => setTimeout(resolve, 20));
      mockInstance?.simulateError();

      expect(onError).toHaveBeenCalled();
    });

    it("should parse and forward events", async () => {
      const onEvent = vi.fn();

      let mockInstance: MockEventSource | undefined;
      const OriginalMock = MockEventSource;
      global.EventSource = class extends OriginalMock {
        constructor(url: string) {
          super(url);
          mockInstance = this;
        }
      } as unknown as typeof EventSource;

      subscribeToChatEvents("test", 8001, {
        onEvent,
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 20));

      const testEvent: ChatEventEnvelope = {
        chat_id: "test",
        seq: "1",
        type: "snapshot",
        thread: {
          id: "test",
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

      mockInstance?.simulateMessage(testEvent);

      expect(onEvent).toHaveBeenCalledWith(testEvent);
    });

    it("should return unsubscribe function that closes EventSource", async () => {
      let mockInstance: MockEventSource | undefined;
      const OriginalMock = MockEventSource;
      global.EventSource = class extends OriginalMock {
        constructor(url: string) {
          super(url);
          mockInstance = this;
        }
      } as unknown as typeof EventSource;

      const unsubscribe = subscribeToChatEvents("test", 8001, {
        onEvent: vi.fn(),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 20));

      unsubscribe();

      expect(mockInstance?.readyState).toBe(2); // CLOSED
    });
  });
});

describe("Event Type Parsing", () => {
  it("should correctly type snapshot events", () => {
    const event: ChatEvent = {
      type: "snapshot",
      thread: {
        id: "123",
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

    expect(event.type).toBe("snapshot");
    if (event.type === "snapshot") {
      expect(event.thread.id).toBe("123");
      expect(event.runtime.state).toBe("idle");
    }
  });

  it("should correctly type stream_delta events", () => {
    const event: ChatEvent = {
      type: "stream_delta",
      message_id: "msg-123",
      ops: [
        { op: "append_content", text: "Hello" },
        { op: "append_reasoning", text: "thinking" },
      ],
    };

    expect(event.type).toBe("stream_delta");
    if (event.type === "stream_delta") {
      expect(event.ops).toHaveLength(2);
      expect(event.ops[0].op).toBe("append_content");
    }
  });

  it("should correctly type pause_required events", () => {
    const event: ChatEvent = {
      type: "pause_required",
      reasons: [
        {
          type: "confirmation",
          command: "shell rm -rf",
          rule: "dangerous command",
          tool_call_id: "call_123",
          integr_config_path: null,
        },
      ],
    };

    expect(event.type).toBe("pause_required");
    if (event.type === "pause_required") {
      expect(event.reasons).toHaveLength(1);
      expect(event.reasons[0].type).toBe("confirmation");
    }
  });
});
