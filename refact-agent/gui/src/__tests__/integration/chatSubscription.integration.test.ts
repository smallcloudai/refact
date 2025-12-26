/**
 * Chat Subscription Integration Tests
 *
 * Integration tests that use the actual refact-lsp server.
 * Requires: refact-lsp running on port 8001
 *
 * Run with: npm run test:no-watch -- chatSubscription.integration
 *
 * Note: These tests are skipped in CI if no server is available.
 */

/* eslint-disable @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-assignment */
import { describe, it, expect, vi } from "vitest";

// Increase test timeout for integration tests
vi.setConfig({ testTimeout: 30000 });
import {
  sendChatCommand,
  sendUserMessage,
  updateChatParams,
  abortGeneration,
} from "../../services/refact/chatCommands";

const LSP_PORT = 8001;
const LSP_URL = `http://127.0.0.1:${LSP_PORT}`;

// Check if server is available
async function isServerAvailable(): Promise<boolean> {
  try {
    const response = await fetch(`${LSP_URL}/v1/ping`, {
      signal: AbortSignal.timeout(2000),
    });
    return response.ok;
  } catch {
    return false;
  }
}

// Generate unique chat ID
function generateChatId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

// Collect events from SSE stream
async function collectEvents(
  chatId: string,
  maxEvents: number,
  timeoutMs: number,
): Promise<unknown[]> {
  const events: unknown[] = [];

  return new Promise((resolve) => {
    const controller = new AbortController();
    const timeout = setTimeout(() => {
      controller.abort();
      resolve(events);
    }, timeoutMs);

    fetch(`${LSP_URL}/v1/chats/subscribe?chat_id=${chatId}`, {
      signal: controller.signal,
    })
      .then(async (response) => {
        const reader = response.body?.getReader();
        if (!reader) {
          clearTimeout(timeout);
          resolve(events);
          return;
        }

        const decoder = new TextDecoder();
        let buffer = "";

        while (events.length < maxEvents) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split("\n");
          buffer = lines.pop() ?? "";

          for (const line of lines) {
            if (line.startsWith("data: ")) {
              try {
                const event = JSON.parse(line.slice(6));
                events.push(event);
                if (events.length >= maxEvents) break;
              } catch {
                // Ignore parse errors
              }
            }
          }
        }

        clearTimeout(timeout);
        controller.abort();
        resolve(events);
      })
      .catch(() => {
        clearTimeout(timeout);
        resolve(events);
      });
  });
}

describe.skipIf(!(await isServerAvailable()))(
  "Chat Subscription Integration Tests",
  () => {
    describe("sendChatCommand", () => {
      it("should accept abort command", async () => {
        const chatId = generateChatId("test-abort");

        await expect(
          sendChatCommand(chatId, LSP_PORT, undefined, { type: "abort" as const })
        ).resolves.toBeUndefined();
      });

      it("should accept set_params command", async () => {
        const chatId = generateChatId("test-params");

        await expect(
          updateChatParams(
            chatId,
            { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
            LSP_PORT,
          )
        ).resolves.toBeUndefined();
      });

      it("should accept user_message command", async () => {
        const chatId = generateChatId("test-message");

        await updateChatParams(
          chatId,
          { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
          LSP_PORT,
        );

        await expect(
          sendUserMessage(
            chatId,
            "Hello, test!",
            LSP_PORT,
          )
        ).resolves.toBeUndefined();
      });

      it("should detect duplicate commands", async () => {
        const chatId = generateChatId("test-duplicate");
        const requestId = `test-${Date.now()}`;

        // First request
        const response1 = await fetch(
          `${LSP_URL}/v1/chats/${chatId}/commands`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              client_request_id: requestId,
              type: "set_params",
              patch: { model: "test" },
            }),
          },
        );

        expect(response1.status).toBe(202);

        // Second request with same ID
        const response2 = await fetch(
          `${LSP_URL}/v1/chats/${chatId}/commands`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              client_request_id: requestId,
              type: "set_params",
              patch: { model: "test" },
            }),
          },
        );

        expect(response2.status).toBe(200);
        const data = await response2.json();
        expect(data.status).toBe("duplicate");
      });
    });

    describe("SSE Subscription", () => {
      it("should receive snapshot on connect", async () => {
        const chatId = generateChatId("test-snapshot");

        const events = await collectEvents(chatId, 1, 5000);

        expect(events.length).toBeGreaterThanOrEqual(1);
        expect(events[0]).toHaveProperty("type", "snapshot");
        expect(events[0]).toHaveProperty("chat_id", chatId);
        expect(events[0]).toHaveProperty("thread");
        expect(events[0]).toHaveProperty("runtime");
        expect(events[0]).toHaveProperty("messages");
      });

      it("should receive events after sending command", async () => {
        const chatId = generateChatId("test-events");

        // Start collecting events
        const eventsPromise = collectEvents(chatId, 10, 10000);

        // Wait a bit for subscription to establish
        await new Promise((r) => setTimeout(r, 300));

        // Send commands
        await updateChatParams(
          chatId,
          { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
          LSP_PORT,
        );

        await sendUserMessage(chatId, "Say hi", LSP_PORT);

        const events = await eventsPromise;

        // Check we got expected events
        const eventTypes = events.map((e: unknown) => (e as { type: string }).type);

        expect(eventTypes).toContain("snapshot");
        expect(eventTypes).toContain("ack"); // Command acknowledgments
      });

      it("should receive stream events during generation", async () => {
        const chatId = generateChatId("test-stream");

        // Start collecting events
        const eventsPromise = collectEvents(chatId, 20, 15000);

        await new Promise((r) => setTimeout(r, 300));

        // Set up chat and send message
        await updateChatParams(
          chatId,
          { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
          LSP_PORT,
        );

        await sendUserMessage(chatId, "Say hello", LSP_PORT);

        const events = await eventsPromise;
        const eventTypes = events.map((e: unknown) => (e as { type: string }).type);

        // Should have streaming events
        expect(eventTypes).toContain("snapshot");
        expect(eventTypes).toContain("message_added"); // User message
        expect(eventTypes).toContain("stream_started");

        // May have stream_delta and stream_finished depending on timing
        // Debug: eventTypes contains the received event types
      });
    });

    describe("Abort Functionality", () => {
      it("should abort generation and receive message_removed", async () => {
        const chatId = generateChatId("test-abort-stream");

        // Start collecting events
        const eventsPromise = collectEvents(chatId, 15, 10000);

        await new Promise((r) => setTimeout(r, 300));

        // Set up chat with a long prompt
        await updateChatParams(
          chatId,
          { model: "refact/claude-haiku-4-5", mode: "NO_TOOLS" },
          LSP_PORT,
        );

        await sendUserMessage(
          chatId,
          "Write a long essay about programming",
          LSP_PORT,
        );

        // Wait for generation to start
        await new Promise((r) => setTimeout(r, 1000));

        // Send abort
        await abortGeneration(chatId, LSP_PORT);

        const events = await eventsPromise;
        const eventTypes = events.map((e: unknown) => (e as { type: string }).type);

        // Debug: eventTypes contains abort test events

        // Should have stream_started and either message_removed (abort) or stream_finished (too late)
        expect(eventTypes).toContain("stream_started");
        expect(
          eventTypes.includes("message_removed") ||
            eventTypes.includes("stream_finished"),
        ).toBe(true);
      });
    });

    describe("Multiple Chats", () => {
      it("should handle multiple independent chats", async () => {
        const chatId1 = generateChatId("test-multi-1");
        const chatId2 = generateChatId("test-multi-2");

        // Connect to both chats
        const events1Promise = collectEvents(chatId1, 5, 8000);
        const events2Promise = collectEvents(chatId2, 5, 8000);

        await new Promise((r) => setTimeout(r, 300));

        // Send different messages to each
        await updateChatParams(
          chatId1,
          { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
          LSP_PORT,
        );
        await updateChatParams(
          chatId2,
          { model: "refact/gpt-4.1-nano", mode: "NO_TOOLS" },
          LSP_PORT,
        );

        await sendUserMessage(chatId1, "Chat 1 message", LSP_PORT);
        await sendUserMessage(chatId2, "Chat 2 message", LSP_PORT);

        const [events1, events2] = await Promise.all([
          events1Promise,
          events2Promise,
        ]);

        // Each should only have events for its own chat
        const chat1Ids = events1.map((e: unknown) => (e as { chat_id: string }).chat_id);
        const chat2Ids = events2.map((e: unknown) => (e as { chat_id: string }).chat_id);

        expect(chat1Ids.every((id: string) => id === chatId1)).toBe(true);
        expect(chat2Ids.every((id: string) => id === chatId2)).toBe(true);
      });
    });
  },
);
