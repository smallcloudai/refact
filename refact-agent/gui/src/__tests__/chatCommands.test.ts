/**
 * Chat Commands Service Tests
 *
 * Tests for the REST API command service.
 * These tests require the refact-lsp server to be running on port 8001.
 *
 * Run with: npm run test:no-watch -- chatCommands
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  sendChatCommand,
  sendUserMessage,
  updateChatParams,
  abortGeneration,
  respondToToolConfirmation,
  sendIdeToolResult,
  type ChatCommand,
  type CommandResponse,
} from "../services/refact/chatCommands";

// Mock fetch for unit tests
const mockFetch = vi.fn();

describe("chatCommands", () => {
  beforeEach(() => {
    global.fetch = mockFetch;
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("sendChatCommand", () => {
    it("should send POST request to correct URL", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      const chatId = "test-chat-123";
      const port = 8001;
      const command: ChatCommand = { type: "abort" };

      await sendChatCommand(chatId, command, port);

      expect(mockFetch).toHaveBeenCalledWith(
        `http://127.0.0.1:${port}/v1/chats/${chatId}/commands`,
        expect.objectContaining({
          method: "POST",
          headers: { "Content-Type": "application/json" },
        }),
      );
    });

    it("should include client_request_id in request body", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      const command: ChatCommand = { type: "abort" };

      await sendChatCommand("test", command, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody).toHaveProperty("client_request_id");
      expect(typeof calledBody.client_request_id).toBe("string");
      expect(calledBody.type).toBe("abort");
    });

    it("should return accepted response", async () => {
      const response: CommandResponse = { status: "accepted" };
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(response),
      });

      const result = await sendChatCommand("test", { type: "abort" }, 8001);

      expect(result).toEqual(response);
    });

    it("should return duplicate response", async () => {
      const response: CommandResponse = { status: "duplicate" };
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(response),
      });

      const result = await sendChatCommand("test", { type: "abort" }, 8001);

      expect(result).toEqual(response);
    });

    it("should throw on HTTP error", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Internal Server Error"),
      });

      await expect(
        sendChatCommand("test", { type: "abort" }, 8001),
      ).rejects.toThrow("Command failed: 500 Internal Server Error");
    });
  });

  describe("sendUserMessage", () => {
    it("should send user_message command with string content", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await sendUserMessage("test-chat", "Hello world", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("user_message");
      expect(calledBody.content).toBe("Hello world");
    });

    it("should send user_message command with multi-modal content", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      const content = [
        { type: "text" as const, text: "What is this?" },
        { type: "image_url" as const, image_url: { url: "data:image/png;base64,..." } },
      ];

      await sendUserMessage("test-chat", content, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("user_message");
      expect(calledBody.content).toEqual(content);
    });

    it("should include attachments if provided", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      const attachments = [{ file: "test.txt" }];
      await sendUserMessage("test-chat", "Hello", 8001, attachments);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.attachments).toEqual(attachments);
    });
  });

  describe("updateChatParams", () => {
    it("should send set_params command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await updateChatParams(
        "test-chat",
        { model: "gpt-4", mode: "AGENT" },
        8001,
      );

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("set_params");
      expect(calledBody.patch).toEqual({ model: "gpt-4", mode: "AGENT" });
    });

    it("should send partial params update", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await updateChatParams("test-chat", { boost_reasoning: true }, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("set_params");
      expect(calledBody.patch).toEqual({ boost_reasoning: true });
    });
  });

  describe("abortGeneration", () => {
    it("should send abort command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await abortGeneration("test-chat", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("abort");
    });
  });

  describe("respondToToolConfirmation", () => {
    it("should send tool_decision command with accepted=true", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await respondToToolConfirmation("test-chat", "call_123", true, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("tool_decision");
      expect(calledBody.tool_call_id).toBe("call_123");
      expect(calledBody.accepted).toBe(true);
    });

    it("should send tool_decision command with accepted=false", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await respondToToolConfirmation("test-chat", "call_456", false, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("tool_decision");
      expect(calledBody.tool_call_id).toBe("call_456");
      expect(calledBody.accepted).toBe(false);
    });
  });

  describe("sendIdeToolResult", () => {
    it("should send ide_tool_result command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await sendIdeToolResult("test-chat", "call_123", "Tool output", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("ide_tool_result");
      expect(calledBody.tool_call_id).toBe("call_123");
      expect(calledBody.content).toBe("Tool output");
      expect(calledBody.tool_failed).toBe(false);
    });

    it("should send ide_tool_result with tool_failed=true", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ status: "accepted" }),
      });

      await sendIdeToolResult(
        "test-chat",
        "call_123",
        "Error occurred",
        8001,
        true,
      );

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.tool_failed).toBe(true);
    });
  });
});

describe("Command Types", () => {
  it("should correctly type user_message command", () => {
    const command: ChatCommand = {
      type: "user_message",
      content: "Hello",
      attachments: [],
    };

    expect(command.type).toBe("user_message");
  });

  it("should correctly type set_params command", () => {
    const command: ChatCommand = {
      type: "set_params",
      patch: {
        model: "gpt-4",
        mode: "AGENT",
        boost_reasoning: true,
      },
    };

    expect(command.type).toBe("set_params");
  });

  it("should correctly type abort command", () => {
    const command: ChatCommand = { type: "abort" };
    expect(command.type).toBe("abort");
  });

  it("should correctly type tool_decision command", () => {
    const command: ChatCommand = {
      type: "tool_decision",
      tool_call_id: "call_123",
      accepted: true,
    };

    expect(command.type).toBe("tool_decision");
  });

  it("should correctly type ide_tool_result command", () => {
    const command: ChatCommand = {
      type: "ide_tool_result",
      tool_call_id: "call_123",
      content: "result",
      tool_failed: false,
    };

    expect(command.type).toBe("ide_tool_result");
  });
});
