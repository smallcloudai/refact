import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { Provider } from "react-redux";
import { configureStore } from "@reduxjs/toolkit";
import { useChatSubscription } from "../hooks/useChatSubscription";
import * as chatSubscriptionModule from "../services/refact/chatSubscription";
import { chatReducer } from "../features/Chat/Thread/reducer";
import { reducer as configReducer } from "../features/Config/configSlice";

vi.mock("../services/refact/chatSubscription");

const createTestStore = () => {
  return configureStore({
    reducer: {
      chat: chatReducer,
      config: configReducer,
    },
  });
};

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <Provider store={createTestStore()}>{children}</Provider>
);

describe("useChatSubscription", () => {
  let mockSubscribe: ReturnType<typeof vi.fn>;
  let mockUnsubscribe: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockUnsubscribe = vi.fn();
    mockSubscribe = vi.fn(() => mockUnsubscribe);
    vi.spyOn(chatSubscriptionModule, "subscribeToChatEvents").mockImplementation(mockSubscribe);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should connect when enabled and chatId present", async () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalledWith(
        "test-chat",
        8001,
        expect.objectContaining({
          onEvent: expect.any(Function),
          onError: expect.any(Function),
        }),
        undefined
      );
    });

    expect(result.current.status).toBe("connecting");
  });

  it("should not connect when disabled", () => {
    renderHook(
      () => useChatSubscription("test-chat", { enabled: false }),
      { wrapper }
    );

    expect(mockSubscribe).not.toHaveBeenCalled();
  });

  it("should not connect when chatId is null", () => {
    renderHook(
      () => useChatSubscription(null, { enabled: true }),
      { wrapper }
    );

    expect(mockSubscribe).not.toHaveBeenCalled();
  });

  it("should dispatch applyChatEvent for valid seq order", async () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    expect(result.current.status).toBe("connected");

    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });
    callbacks.onEvent({ chat_id: "test-chat", seq: "2", type: "pause_cleared" });

    expect(result.current.lastSeq).toBe("2");
  });

  it("should ignore duplicate seq", async () => {
    const onEvent = vi.fn();
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, onEvent }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });
    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });

    expect(result.current.lastSeq).toBe("1");
    expect(onEvent).toHaveBeenCalledTimes(2);
  });

  it("should ignore out-of-order seq", async () => {
    const onEvent = vi.fn();
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, onEvent }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "2", type: "pause_cleared" });
    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });

    expect(result.current.lastSeq).toBe("0");
    expect(onEvent).toHaveBeenCalledTimes(1);
  });

  it("should reconnect on seq gap when autoReconnect enabled", async () => {
    vi.useFakeTimers();

    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, autoReconnect: true }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "5", type: "pause_cleared" });

    expect(mockUnsubscribe).toHaveBeenCalled();
    expect(result.current.status).toBe("disconnected");

    vi.runAllTimers();

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalledTimes(2);
    });

    vi.useRealTimers();
  });

  it("should not reconnect on seq gap when autoReconnect disabled", async () => {
    vi.useFakeTimers();

    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, autoReconnect: false }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "5", type: "pause_cleared" });

    expect(mockUnsubscribe).toHaveBeenCalled();
    expect(result.current.status).toBe("disconnected");

    vi.runAllTimers();

    expect(mockSubscribe).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });

  it("should reconnect on error with delay", async () => {
    vi.useFakeTimers();

    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, autoReconnect: true, reconnectDelay: 2000 }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onError(new Error("Connection failed"));

    expect(result.current.status).toBe("disconnected");
    expect(result.current.error?.message).toBe("Connection failed");

    vi.advanceTimersByTime(1000);
    expect(mockSubscribe).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(1000);

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalledTimes(2);
    });

    vi.useRealTimers();
  });

  it("should not reconnect on error when autoReconnect disabled", async () => {
    vi.useFakeTimers();

    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true, autoReconnect: false }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onError(new Error("Connection failed"));

    expect(result.current.status).toBe("disconnected");

    vi.runAllTimers();

    expect(mockSubscribe).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });

  it("should cleanup on unmount", async () => {
    const { unmount } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    unmount();

    expect(mockUnsubscribe).toHaveBeenCalled();
  });

  it("should prevent concurrent connections", async () => {
    const { rerender } = renderHook(
      ({ chatId }) => useChatSubscription(chatId, { enabled: true }),
      { wrapper, initialProps: { chatId: "test-chat-1" } }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalledTimes(1);
    });

    rerender({ chatId: "test-chat-2" });

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalledTimes(2);
    });

    expect(mockUnsubscribe).toHaveBeenCalledTimes(1);
  });

  it("should call custom callbacks", async () => {
    const onConnected = vi.fn();
    const onDisconnected = vi.fn();
    const onError = vi.fn();
    const onEvent = vi.fn();

    renderHook(
      () => useChatSubscription("test-chat", {
        enabled: true,
        onConnected,
        onDisconnected,
        onError,
        onEvent,
      }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    expect(onConnected).toHaveBeenCalled();

    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    expect(onEvent).toHaveBeenCalled();

    callbacks.onDisconnected();
    expect(onDisconnected).toHaveBeenCalled();

    callbacks.onError(new Error("Test error"));
    expect(onError).toHaveBeenCalled();
  });

  it("should reset seq on snapshot", async () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: true }),
      { wrapper }
    );

    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    const callbacks = mockSubscribe.mock.calls[0][2];

    callbacks.onConnected();
    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });
    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });
    callbacks.onEvent({ chat_id: "test-chat", seq: "2", type: "pause_cleared" });

    expect(result.current.lastSeq).toBe("2");

    callbacks.onEvent({ chat_id: "test-chat", seq: "0", type: "snapshot", thread: {}, runtime: {}, messages: [] });

    expect(result.current.lastSeq).toBe("0");

    callbacks.onEvent({ chat_id: "test-chat", seq: "1", type: "pause_cleared" });

    expect(result.current.lastSeq).toBe("1");
  });
});
