import { describe, expect, test } from "vitest";
import { reducer, createInitialState } from ".";
import { EVENT_NAMES_TO_CHAT, ToolCall, ResponseToChat } from "../../events";
import { mergeToolCalls } from "./utils";

describe("reducer", () => {
  test("handle an empty message from the assistant", () => {
    const initialState = createInitialState();
    const action: ResponseToChat = {
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

    expect(() =>
      reducer(window.postMessage)(initialState, action),
    ).not.toThrow();
  });
});

describe("mergeToolCalls", () => {
  test("combines two tool calls", () => {
    const stored: ToolCall[] = [
      {
        function: {
          arguments: "",
          name: "definition",
        },
        id: "call_8Btwv94t0eH60msyRQHFCxyU",
        index: 0,
        type: "function",
      },
    ];
    const toAdd: ToolCall[] = [
      {
        function: {
          arguments: '{"',
        },
        index: 0,
      },
    ];

    const expected = [
      {
        function: {
          arguments: '{"',
          name: "definition",
        },
        id: "call_8Btwv94t0eH60msyRQHFCxyU",
        index: 0,
        type: "function",
      },
    ];

    const result = mergeToolCalls(stored, toAdd);

    expect(result).toEqual(expected);
  });
});
