import { describe, test, expect } from "vitest";
import { isAssistantDelta, isToolCallDelta } from "./types";

const TOOL_DELTAS = [
  // Refact
  {
    content: "performed vecdb search for query: frog",
    role: "tool",
    tool_call_id: "call_xv5LsVa25XcPsF4L28kOZKxt",
    tool_calls: null,
  },
  {
    content: "performed vecdb search for query: frog.Frog",
    role: "tool",
    tool_call_id: "call_BFtgiaVtfBMCKLw3cNJghhpj",
    tool_calls: null,
  },
  // Openai
  {
    content: "`frog` (defined in jump_to_conclusions.py and other files)",
    role: "tool",
    tool_call_id: "call_VceeyHswqT1Gj0WfXy5GxYvY",
    tool_calls: null,
  },
];

const ASSISTANT_DELTAS = [
  // Refact
  {
    content: "The",
    function_call: null,
    role: "assistant",
    tool_calls: null,
  },
  {
    content: " project",
    function_call: null,
    role: null,
    tool_calls: null,
  },
  // Openai
  {
    content: "",
    role: "assistant",
  },
  {
    content: "This",
  },
];

const TOOL_CALL_DELTAS = [
  // Refact
  {
    content: null,
    function_call: null,
    role: "assistant",
    tool_calls: [
      {
        function: {
          arguments: "",
          name: "search_workspace",
        },
        id: "call_xv5LsVa25XcPsF4L28kOZKxt",
        index: 0,
        type: "function",
      },
    ],
  },
  {
    content: null,
    function_call: null,
    role: "assistant",
    tool_calls: [
      {
        function: {
          arguments: '{"qu',
          name: null,
        },
        id: null,
        index: 0,
        type: "function",
      },
    ],
  },
  // Openai
  {
    content: null,
    role: "assistant",
    tool_calls: [
      {
        function: {
          arguments: "",
          name: "definition",
        },
        id: "call_VceeyHswqT1Gj0WfXy5GxYvY",
        index: 0,
        type: "function",
      },
    ],
  },
  {
    tool_calls: [
      {
        function: {
          arguments: '{"symbol:"frog"}',
        },
        index: 0,
      },
    ],
  },
];

const EMPTY_DELTAS = [
  {
    content: null,
    function_call: null,
    role: null,
    tool_calls: null,
  },
  // openai
  {},
];

describe("type-guards", () => {
  test.each(TOOL_CALL_DELTAS)("isToolCallDelta should be true", (toolCall) => {
    expect(isToolCallDelta(toolCall)).toBe(true);
  });

  test.each([...ASSISTANT_DELTAS, ...EMPTY_DELTAS, ...TOOL_DELTAS])(
    "isToolCallDelta should be false",
    (toolCall) => {
      expect(isToolCallDelta(toolCall)).toBe(false);
    },
  );

  test.each(ASSISTANT_DELTAS)("isAssistantDelta should be true", (toolCall) => {
    expect(isAssistantDelta(toolCall)).toBe(true);
  });

  test.each([...TOOL_CALL_DELTAS, ...EMPTY_DELTAS, ...TOOL_DELTAS])(
    "isAssistantDelta should be false",
    (toolCall) => {
      expect(isAssistantDelta(toolCall)).toBe(false);
    },
  );
});
