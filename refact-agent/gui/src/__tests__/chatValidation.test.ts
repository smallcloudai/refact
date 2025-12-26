/* eslint-disable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-argument */
import { describe, test, expect } from "vitest";
import { isLspChatMessage } from "../services/refact/chat";
import { applyDeltaOps } from "../services/refact/chatSubscription";
import type { ChatMessage } from "../services/refact/types";

describe("Chat Validation Fixes", () => {
  describe("isLspChatMessage - tool messages", () => {
    test("accepts tool message with string content", () => {
      const msg = {
        role: "tool",
        tool_call_id: "call_123",
        content: "Tool result text",
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });

    test("accepts tool message with array content", () => {
      const msg = {
        role: "tool",
        tool_call_id: "call_123",
        content: [
          { m_type: "text", m_content: "Result text" },
          { m_type: "image/png", m_content: "base64data" },
        ],
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });

    test("rejects tool message without tool_call_id", () => {
      const msg = {
        role: "tool",
        content: "Some text",
      };
      expect(isLspChatMessage(msg)).toBe(false);
    });
  });

  describe("isLspChatMessage - diff messages", () => {
    test("accepts diff message with array content", () => {
      const msg = {
        role: "diff",
        content: [
          {
            file_name: "test.ts",
            file_action: "M",
            line1: 1,
            line2: 10,
            chunks: "diff content",
          },
        ],
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });

    test("rejects diff message with non-array content", () => {
      const msg = {
        role: "diff",
        content: "not an array",
      };
      expect(isLspChatMessage(msg)).toBe(false);
    });
  });

  describe("isLspChatMessage - multimodal user messages", () => {
    test("accepts user message with array content", () => {
      const msg = {
        role: "user",
        content: [
          { type: "text", text: "What is this?" },
          { type: "image_url", image_url: { url: "data:image/png;base64,..." } },
        ],
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });
  });

  describe("isLspChatMessage - standard messages", () => {
    test("accepts assistant message with null content", () => {
      const msg = {
        role: "assistant",
        content: null,
        tool_calls: [{ id: "call_1", function: { name: "test", arguments: "{}" }, index: 0 }],
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });

    test("accepts assistant message with string content", () => {
      const msg = {
        role: "assistant",
        content: "Hello world",
      };
      expect(isLspChatMessage(msg)).toBe(true);
    });
  });
});

describe("applyDeltaOps - merge_extra", () => {
  test("merges extra fields into message", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg_1",
    };

    const result = applyDeltaOps(message, [
      { op: "merge_extra", extra: { custom_field: "value1" } },
    ]);

    expect(result).toHaveProperty("extra");
    expect((result as any).extra.custom_field).toBe("value1");
  });

  test("preserves existing extra fields when merging", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg_1",
      extra: { existing: "kept" },
    } as any;

    const result = applyDeltaOps(message, [
      { op: "merge_extra", extra: { new_field: "added" } },
    ]);

    expect((result as any).extra.existing).toBe("kept");
    expect((result as any).extra.new_field).toBe("added");
  });

  test("overwrites existing extra fields with same key", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg_1",
      extra: { field: "old" },
    } as any;

    const result = applyDeltaOps(message, [
      { op: "merge_extra", extra: { field: "new" } },
    ]);

    expect((result as any).extra.field).toBe("new");
  });

  test("handles unknown delta ops gracefully", () => {
    const message: ChatMessage = {
      role: "assistant",
      content: "test",
      message_id: "msg_1",
    };

    const result = applyDeltaOps(message, [
      { op: "unknown_op" } as any,
    ]);

    expect(result).toBeDefined();
    expect(result.content).toBe("test");
  });
});
