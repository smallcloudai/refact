/**
 * SSE Protocol Corner Cases Tests
 *
 * Tests chunking, sequence gaps, disconnects, and message variations
 *
 * Run with: npm run test:no-watch -- chatSSEProtocolCornerCases
 */

/* eslint-disable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/require-await, @typescript-eslint/ban-ts-comment */
// @ts-nocheck - Testing runtime behavior
import { describe, it, expect, vi, beforeEach } from "vitest";
import { subscribeToChatEvents } from "../services/refact/chatSubscription";

const createMockReader = (chunks: Uint8Array[]) => {
  let index = 0;
  return {
    read: vi.fn(async () => {
      if (index >= chunks.length) {
        return { done: true, value: undefined };
      }
      return { done: false, value: chunks[index++] };
    }),
  };
};

const createMockFetch = (chunks: Uint8Array[]) => {
  return vi.fn().mockResolvedValue({
    ok: true,
    body: {
      getReader: () => createMockReader(chunks),
    },
  });
};

describe("SSE Protocol - Chunking Corner Cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should handle JSON split across chunks", async () => {
    const encoder = new TextEncoder();
    const fullEvent = `data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`;
    
    const chunk1 = encoder.encode(fullEvent.substring(0, 30));
    const chunk2 = encoder.encode(fullEvent.substring(30));

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk1, chunk2]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle delimiter split across chunks", async () => {
    const encoder = new TextEncoder();
    const event = JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" });
    
    const chunk1 = encoder.encode(`data: ${event}\n`);
    const chunk2 = encoder.encode(`\n`);

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk1, chunk2]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle CRLF split across chunks", async () => {
    const encoder = new TextEncoder();
    const event = JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" });
    
    const chunk1 = encoder.encode(`data: ${event}\r`);
    const chunk2 = encoder.encode(`\n\r\n`);

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk1, chunk2]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle CR-only line endings", async () => {
    const encoder = new TextEncoder();
    const event = JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" });
    
    const chunk = encoder.encode(`data: ${event}\r\r`);

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle multiple events in one chunk", async () => {
    const encoder = new TextEncoder();
    const event1 = JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" });
    const event2 = JSON.stringify({ chat_id: "test", seq: "2", type: "pause_cleared" });
    
    const chunk = encoder.encode(`data: ${event1}\n\ndata: ${event2}\n\n`);

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(2);
    expect(events[0].seq).toBe("1");
    expect(events[1].seq).toBe("2");
  });

  it("should handle empty lines between events", async () => {
    const encoder = new TextEncoder();
    const event = JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" });
    
    const chunk = encoder.encode(`\n\ndata: ${event}\n\n\n\n`);

    const events: any[] = [];
    const mockFetch = createMockFetch([chunk]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pause_cleared");
  });

  it("should handle large payload across many chunks", async () => {
    const encoder = new TextEncoder();
    const largeContent = "x".repeat(10000);
    const event = JSON.stringify({ 
      chat_id: "test", 
      seq: "1", 
      type: "stream_delta",
      message_id: "msg-1",
      ops: [{ op: "append_content", text: largeContent }]
    });
    const fullEvent = `data: ${event}\n\n`;
    
    const chunkSize = 100;
    const chunks: Uint8Array[] = [];
    for (let i = 0; i < fullEvent.length; i += chunkSize) {
      chunks.push(encoder.encode(fullEvent.substring(i, i + chunkSize)));
    }

    const events: any[] = [];
    const mockFetch = createMockFetch(chunks);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("stream_delta");
    expect(events[0].ops[0].text).toBe(largeContent);
  });
});

describe("SSE Protocol - Message Variations", () => {
  it("should handle context_file message in snapshot", async () => {
    const encoder = new TextEncoder();
    const snapshot = {
      chat_id: "test",
      seq: "0",
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
        pause_reasons: [],
      },
      messages: [
        {
          role: "context_file",
          content: [
            {
              file_name: "test.ts",
              file_content: "console.log('test');",
              line1: 1,
              line2: 1,
            },
          ],
        },
      ],
    };

    const events: any[] = [];
    const mockFetch = createMockFetch([encoder.encode(`data: ${JSON.stringify(snapshot)}\n\n`)]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events[0].messages).toHaveLength(1);
    expect(events[0].messages[0].role).toBe("context_file");
    expect(Array.isArray(events[0].messages[0].content)).toBe(true);
  });

  it("should handle assistant message with all optional fields", async () => {
    const encoder = new TextEncoder();
    const snapshot = {
      chat_id: "test",
      seq: "0",
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
        pause_reasons: [],
      },
      messages: [
        {
          role: "assistant",
          content: "Test response",
          message_id: "msg-1",
          reasoning_content: "Let me think...",
          thinking_blocks: [{ thinking: "Step 1", signature: "sig1" }],
          citations: [{ url: "http://example.com", title: "Example" }],
          usage: { prompt_tokens: 100, completion_tokens: 50, total_tokens: 150 },
          extra: { custom_field: "value" },
          finish_reason: "stop",
        },
      ],
    };

    const events: any[] = [];
    const mockFetch = createMockFetch([encoder.encode(`data: ${JSON.stringify(snapshot)}\n\n`)]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    const msg = events[0].messages[0];
    expect(msg.reasoning_content).toBe("Let me think...");
    expect(msg.thinking_blocks).toHaveLength(1);
    expect(msg.citations).toHaveLength(1);
    expect(msg.usage.total_tokens).toBe(150);
    expect(msg.extra.custom_field).toBe("value");
  });

  it("should handle tool message with tool_failed variations", async () => {
    const encoder = new TextEncoder();
    
    for (const toolFailed of [true, false, null, undefined]) {
      const snapshot = {
        chat_id: "test",
        seq: "0",
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
          pause_reasons: [],
        },
        messages: [
          {
            role: "tool",
            content: "Result",
            message_id: "msg-1",
            tool_call_id: "call_1",
            tool_failed: toolFailed,
          },
        ],
      };

      const events: any[] = [];
      const mockFetch = createMockFetch([encoder.encode(`data: ${JSON.stringify(snapshot)}\n\n`)]);
      global.fetch = mockFetch;

      subscribeToChatEvents("test", 8001, {
        onEvent: (e) => events.push(e),
        onError: vi.fn(),
      });

      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events[0].messages[0].tool_failed).toBe(toolFailed);
    }
  });

  it("should handle multimodal tool message content", async () => {
    const encoder = new TextEncoder();
    const snapshot = {
      chat_id: "test",
      seq: "0",
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
        pause_reasons: [],
      },
      messages: [
        {
          role: "tool",
          content: [
            { m_type: "text", m_content: "Result text" },
            { m_type: "image/png", m_content: "base64data..." },
          ],
          message_id: "msg-1",
          tool_call_id: "call_1",
        },
      ],
    };

    const events: any[] = [];
    const mockFetch = createMockFetch([encoder.encode(`data: ${JSON.stringify(snapshot)}\n\n`)]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: (e) => events.push(e),
      onError: vi.fn(),
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    const content = events[0].messages[0].content;
    expect(Array.isArray(content)).toBe(true);
    expect(content[0].m_type).toBe("text");
    expect(content[1].m_type).toBe("image/png");
  });
});

describe("SSE Protocol - Disconnect Handling", () => {
  it("should call onDisconnected on normal EOF", async () => {
    const onDisconnected = vi.fn();
    const encoder = new TextEncoder();
    
    const mockFetch = createMockFetch([
      encoder.encode(`data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`),
    ]);
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: vi.fn(),
      onError: vi.fn(),
      onDisconnected,
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onDisconnected).toHaveBeenCalled();
  });

  it("should call onError on fetch error", async () => {
    const onError = vi.fn();
    
    const mockFetch = vi.fn().mockRejectedValue(new Error("Network error"));
    global.fetch = mockFetch;

    subscribeToChatEvents("test", 8001, {
      onEvent: vi.fn(),
      onError,
    });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onError).toHaveBeenCalled();
  });

  it("should not call onDisconnected on abort", async () => {
    const onDisconnected = vi.fn();
    const encoder = new TextEncoder();
    
    const _abortFn: (() => void) | null = null;
    const mockFetch = vi.fn().mockImplementation((url, options) => {
      const abortController = options.signal;
      
      return Promise.resolve({
        ok: true,
        body: {
          getReader: () => ({
            read: vi.fn().mockImplementation(async () => {
              if (abortController.aborted) {
                throw new DOMException("Aborted", "AbortError");
              }
              await new Promise((resolve) => setTimeout(resolve, 100));
              return { done: false, value: encoder.encode(`data: ${JSON.stringify({ chat_id: "test", seq: "1", type: "pause_cleared" })}\n\n`) };
            }),
          }),
        },
      });
    });
    global.fetch = mockFetch;

    const unsubscribe = subscribeToChatEvents("test", 8001, {
      onEvent: vi.fn(),
      onError: vi.fn(),
      onDisconnected,
    });

    await new Promise((resolve) => setTimeout(resolve, 5));
    unsubscribe();
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onDisconnected).toHaveBeenCalledTimes(1);
  });
});
