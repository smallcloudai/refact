import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { Provider } from "react-redux";
import { configureStore } from "@reduxjs/toolkit";
import { useChatSubscription } from "../hooks/useChatSubscription";
import { chatReducer } from "../features/Chat/Thread/reducer";
import { reducer as configReducer } from "../features/Config/configSlice";

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
  afterEach(() => {
    vi.useRealTimers();
  });

  it("should return disconnected status when disabled", () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: false }),
      { wrapper }
    );

    expect(result.current.status).toBe("disconnected");
    expect(result.current.isConnected).toBe(false);
    expect(result.current.isConnecting).toBe(false);
  });

  it("should return disconnected status when chatId is null", () => {
    const { result } = renderHook(
      () => useChatSubscription(null, { enabled: true }),
      { wrapper }
    );

    expect(result.current.status).toBe("disconnected");
  });

  it("should return disconnected status when chatId is undefined", () => {
    const { result } = renderHook(
      () => useChatSubscription(undefined, { enabled: true }),
      { wrapper }
    );

    expect(result.current.status).toBe("disconnected");
  });

  it("should have connect and disconnect functions", () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: false }),
      { wrapper }
    );

    expect(typeof result.current.connect).toBe("function");
    expect(typeof result.current.disconnect).toBe("function");
  });

  it("should have lastSeq as string", () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: false }),
      { wrapper }
    );

    expect(typeof result.current.lastSeq).toBe("string");
    expect(result.current.lastSeq).toBe("0");
  });

  it("should have null error initially", () => {
    const { result } = renderHook(
      () => useChatSubscription("test-chat", { enabled: false }),
      { wrapper }
    );

    expect(result.current.error).toBeNull();
  });
});
