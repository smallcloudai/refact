import { expect, test, describe } from "vitest";
import { chatReducer } from "./reducer";
import { chatResponse, newChatAction } from "./actions";

describe("Chat Thread Reducer", () => {
  test("streaming should be true on any response", () => {
    // Create initial empty state and then add a new thread
    const emptyState = chatReducer(undefined, { type: "@@INIT" });
    const stateWithThread = chatReducer(emptyState, newChatAction(undefined));
    const chatId = stateWithThread.current_thread_id;

    const msg = chatResponse({
      id: chatId,
      role: "tool",
      tool_call_id: "test_tool",
      content: "👀",
    });

    const result = chatReducer(stateWithThread, msg);
    expect(result.threads[chatId]?.streaming).toEqual(true);
  });
});
