import { expect, test, describe } from "vitest";
import { chatReducer } from "./reducer";
import { chatResponse } from "./actions";
import { createAction } from "@reduxjs/toolkit";

describe("Chat Thread Reducer", () => {
  test("streaming should be true on any response", () => {
    const init = chatReducer(undefined, createAction("noop")());
    const msg = chatResponse({
      id: init.thread.id,
      ftm_role: "tool",
      tool_call_id: "test_tool",
      ftm_content: "👀",
    });

    const result = chatReducer(init, msg);
    expect(result.streaming).toEqual(true);
  });
});
