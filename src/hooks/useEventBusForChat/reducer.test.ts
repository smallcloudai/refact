import { describe, expect, test } from "vitest";
import { reducer, createInitialState } from ".";
import { EVENT_NAMES_TO_CHAT } from "../../events";

describe("reducer", () => {
  test("handle an empty message from the assistant", () => {
    const initialState = createInitialState();
    const action = {
      type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
      payload: {
        id: initialState.chat.id,
        choices: [
          {
            delta: {
              content: "",
              role: "assistant",
            },
            finish_reason: null,
            index: 0,
          },
        ],
        created: 1710777171.188,
        model: "gpt-3.5-turbo",
      },
    };

    expect(() => reducer(initialState, action)).not.toThrow();
  });
});
