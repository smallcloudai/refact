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
  respondToToolConfirmations,
  updateMessage,
  removeMessage,
  type ChatCommand,
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
      });

      const chatId = "test-chat-123";
      const port = 8001;
      const command = { type: "abort" as const };

      await sendChatCommand(chatId, port, undefined, command);

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
      });

      const command = { type: "abort" as const };

      await sendChatCommand("test", 8001, undefined, command);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody).toHaveProperty("client_request_id");
      expect(typeof calledBody.client_request_id).toBe("string");
      expect(calledBody.type).toBe("abort");
    });

    it("should include authorization header when apiKey provided", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await sendChatCommand("test", 8001, "test-key", { type: "abort" as const });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            "Authorization": "Bearer test-key",
          }),
        }),
      );
    });

    it("should throw on HTTP error", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
        text: () => Promise.resolve("Error details"),
      });

      await expect(
        sendChatCommand("test", 8001, undefined, { type: "abort" as const }),
      ).rejects.toThrow("Failed to send command");
    });
  });

  describe("sendUserMessage", () => {
    it("should send user_message command with string content", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await sendUserMessage("test-chat", "Hello world", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("user_message");
      expect(calledBody.content).toBe("Hello world");
    });

    it("should send user_message command with multi-modal content", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      const content = [
        { type: "text" as const, text: "What is this?" },
        { type: "image_url" as const, image_url: { url: "data:image/png;base64,..." } },
      ];

      await sendUserMessage("test-chat", content, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("user_message");
      expect(Array.isArray(calledBody.content)).toBe(true);
      expect(calledBody.content).toEqual(content);
    });
  });

  describe("updateChatParams", () => {
    it("should send set_params command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
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
      });

      await respondToToolConfirmation("test-chat", "call_456", false, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("tool_decision");
      expect(calledBody.tool_call_id).toBe("call_456");
      expect(calledBody.accepted).toBe(false);
    });
  });

  describe("respondToToolConfirmations", () => {
    it("should send tool_decisions command with object array", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      const decisions = [
        { tool_call_id: "call_1", accepted: true },
        { tool_call_id: "call_2", accepted: false },
        { tool_call_id: "call_3", accepted: true },
      ];

      await respondToToolConfirmations("test-chat", decisions, 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("tool_decisions");
      expect(calledBody.decisions).toEqual(decisions);
      expect(Array.isArray(calledBody.decisions)).toBe(true);
      expect(calledBody.decisions[0]).toHaveProperty("tool_call_id");
      expect(calledBody.decisions[0]).toHaveProperty("accepted");
    });
  });

  describe("updateMessage", () => {
    it("should send update_message command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await updateMessage("test-chat", "msg_5", "Updated text", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("update_message");
      expect(calledBody.message_id).toBe("msg_5");
      expect(calledBody.content).toBe("Updated text");
    });

    it("should send update_message with regenerate flag", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await updateMessage("test-chat", "msg_5", "Updated text", 8001, undefined, true);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("update_message");
      expect(calledBody.regenerate).toBe(true);
    });
  });

  describe("removeMessage", () => {
    it("should send remove_message command", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await removeMessage("test-chat", "msg_5", 8001);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("remove_message");
      expect(calledBody.message_id).toBe("msg_5");
    });

    it("should send remove_message with regenerate flag", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      await removeMessage("test-chat", "msg_5", 8001, undefined, true);

      const calledBody = JSON.parse(mockFetch.mock.calls[0][1].body);
      expect(calledBody.type).toBe("remove_message");
      expect(calledBody.regenerate).toBe(true);
    });
  });
});

describe("Command Types", () => {
  it("should correctly type user_message command with string", () => {
    const command: ChatCommand = {
      type: "user_message",
      content: "Hello",
      attachments: [],
      client_request_id: "test-id",
    };

    expect(command.type).toBe("user_message");
  });

  it("should correctly type user_message command with multimodal array", () => {
    const command: ChatCommand = {
      type: "user_message",
      content: [
        { type: "text", text: "Hello" },
        { type: "image_url", image_url: { url: "data:..." } },
      ],
      attachments: [],
      client_request_id: "test-id",
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
      client_request_id: "test-id",
    };

    expect(command.type).toBe("set_params");
  });

  it("should correctly type abort command", () => {
    const command: ChatCommand = { 
      type: "abort",
      client_request_id: "test-id",
    };
    expect(command.type).toBe("abort");
  });

  it("should correctly type tool_decision command", () => {
    const command: ChatCommand = {
      type: "tool_decision",
      tool_call_id: "call_123",
      accepted: true,
      client_request_id: "test-id",
    };

    expect(command.type).toBe("tool_decision");
  });

  it("should correctly type ide_tool_result command", () => {
    const command: ChatCommand = {
      type: "ide_tool_result",
      tool_call_id: "call_123",
      content: "result",
      tool_failed: false,
      client_request_id: "test-id",
    };

    expect(command.type).toBe("ide_tool_result");
  });

  it("should correctly type tool_decisions command", () => {
    const command: ChatCommand = {
      type: "tool_decisions",
      decisions: [
        { tool_call_id: "call_1", accepted: true },
        { tool_call_id: "call_2", accepted: false },
      ],
      client_request_id: "test-id",
    };

    expect(command.type).toBe("tool_decisions");
  });

  it("should correctly type update_message command", () => {
    const command: ChatCommand = {
      type: "update_message",
      message_id: "msg_5",
      content: "Updated",
      regenerate: true,
      client_request_id: "test-id",
    };

    expect(command.type).toBe("update_message");
  });

  it("should correctly type remove_message command", () => {
    const command: ChatCommand = {
      type: "remove_message",
      message_id: "msg_5",
      regenerate: false,
      client_request_id: "test-id",
    };

    expect(command.type).toBe("remove_message");
  });
});
