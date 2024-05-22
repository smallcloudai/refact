import { describe, expect, test } from "vitest";
import { reducer, createInitialState } from ".";
import { ChatMessages, EVENT_NAMES_TO_CHAT, ToolCall } from "../../events";
import { appendToolCallsToAssistantMessage } from "./appendToolCallsToAssistantMessage";

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

    expect(() =>
      reducer(window.postMessage)(initialState, action),
    ).not.toThrow();
  });
});

describe("appendToolCallsToAssistantMessage", () => {
  test("if messages does not have tool_calls it should return the messages", () => {
    const messages: ChatMessages = [
      ["user", "hello"],
      ["assistant", "hello there"],
      ["user", "how are you?"],
      ["assistant", "I'm good"],
    ];

    const expected = messages;
    const result = appendToolCallsToAssistantMessage(messages);

    expect(result).toEqual(expected);
  });

  test("if message does have tool_calls it should add the tool calls to the previouse assistant message", () => {
    const toolCalls: ToolCall[] = [
      {
        id: "a",
        function: {
          arguments: { file: "meow.txt" },
          name: "cat",
        },
        type: "function",
      },
    ];
    const messages: ChatMessages = [
      ["user", "hello"],
      ["assistant", "hello there"],
      ["tool_calls", toolCalls],
      ["user", "how are you?"],
      ["assistant", "I'm good"],
    ];

    const expected = [
      ["user", "hello"],
      ["assistant", "hello there", toolCalls],
      ["user", "how are you?"],
      ["assistant", "I'm good"],
    ];

    const result = appendToolCallsToAssistantMessage(messages);

    expect(result).toEqual(expected);
  });
});
