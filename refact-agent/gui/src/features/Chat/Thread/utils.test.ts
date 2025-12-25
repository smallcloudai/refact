import { describe, expect, test } from "vitest";
import {
  ChatMessages,
  type ToolCall,
} from "../../../services/refact";
import {
  mergeToolCalls,
  postProcessMessagesAfterStreaming,
} from "./utils";

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

describe("postProcessMessagesAfterStreaming", () => {
  test("should filter out server-executed tool calls and store in server_executed_tools", () => {
    const messages: ChatMessages = [
      {
        role: "assistant",
        content: "I'll search for the weather.",
        tool_calls: [
          {
            id: "srvtoolu_123",
            index: 0,
            function: {
              name: "web_search",
              arguments: '{"query": "weather in Adelaide"}',
            },
          },
          {
            id: "call_456",
            index: 1,
            function: {
              name: "str_replace",
              arguments: '{"old": "a", "new": "b"}',
            },
          },
        ],
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toHaveLength(1);
    expect(result[0].role).toBe("assistant");
    if (
      "tool_calls" in result[0] &&
      "content" in result[0] &&
      "server_executed_tools" in result[0]
    ) {
      expect(result[0].tool_calls).toHaveLength(1);
      expect(result[0].tool_calls?.[0].function.name).toBe("str_replace");
      expect(result[0].content).toBe("I'll search for the weather.");
      expect(result[0].server_executed_tools).toHaveLength(1);
      expect(result[0].server_executed_tools?.[0].function.name).toBe(
        "web_search",
      );
    }
  });

  test("should remove tool_calls when all are server-executed and store in server_executed_tools", () => {
    const messages: ChatMessages = [
      {
        role: "assistant",
        content: "Searching for information.",
        tool_calls: [
          {
            id: "srvtoolu_123",
            index: 0,
            function: {
              name: "web_search",
              arguments: '{"query": "test"}',
            },
          },
        ],
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toHaveLength(1);
    expect(result[0].role).toBe("assistant");
    if ("content" in result[0] && "server_executed_tools" in result[0]) {
      expect(result[0].content).toBe("Searching for information.");
      expect(result[0].server_executed_tools).toHaveLength(1);
      expect(result[0].server_executed_tools?.[0].function.name).toBe(
        "web_search",
      );
    }
    if ("tool_calls" in result[0]) {
      expect(result[0].tool_calls).toBeUndefined();
    }
  });

  test("should not modify messages without tool_calls", () => {
    const messages: ChatMessages = [
      {
        role: "user",
        content: "Hello",
        checkpoints: [],
      },
      {
        role: "assistant",
        content: "Hi there!",
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toEqual(messages);
  });

  test("should not modify messages with non-filtered tools", () => {
    const messages: ChatMessages = [
      {
        role: "assistant",
        content: "I'll replace that for you.",
        tool_calls: [
          {
            id: "call_456",
            index: 0,
            function: {
              name: "str_replace",
              arguments: '{"old": "a", "new": "b"}',
            },
          },
        ],
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toEqual(messages);
  });

  test("should deduplicate tool calls with same ID, keeping the one with arguments", () => {
    const messages: ChatMessages = [
      {
        role: "assistant",
        content: "Processing your request.",
        tool_calls: [
          {
            id: "call_123",
            index: 0,
            function: {
              name: "tree",
              arguments: "",
            },
          },
          {
            id: "call_123",
            index: 1,
            function: {
              name: "tree",
              arguments: '{"path": "/src"}',
            },
          },
          {
            id: "call_456",
            index: 2,
            function: {
              name: "cat",
              arguments: '{"file": "test.js"}',
            },
          },
        ],
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toHaveLength(1);
    expect(result[0].role).toBe("assistant");
    if ("tool_calls" in result[0]) {
      expect(result[0].tool_calls).toHaveLength(2);
      expect(result[0].tool_calls?.[0].id).toBe("call_123");
      expect(result[0].tool_calls?.[0].function.arguments).toBe(
        '{"path": "/src"}',
      );
      expect(result[0].tool_calls?.[1].id).toBe("call_456");
    }
  });

  test("should handle deduplication and filtering together", () => {
    const messages: ChatMessages = [
      {
        role: "assistant",
        content: "Let me search and check the files.",
        tool_calls: [
          {
            id: "srvtoolu_123",
            index: 0,
            function: {
              name: "web_search",
              arguments: "",
            },
          },
          {
            id: "srvtoolu_123",
            index: 1,
            function: {
              name: "web_search",
              arguments: '{"query": "test search"}',
            },
          },
          {
            id: "call_456",
            index: 2,
            function: {
              name: "tree",
              arguments: "",
            },
          },
          {
            id: "call_456",
            index: 3,
            function: {
              name: "tree",
              arguments: '{"path": "/"}',
            },
          },
        ],
      },
    ];

    const result = postProcessMessagesAfterStreaming(messages);

    expect(result).toHaveLength(1);
    if (
      "tool_calls" in result[0] &&
      "content" in result[0] &&
      "server_executed_tools" in result[0]
    ) {
      expect(result[0].tool_calls).toHaveLength(1);
      expect(result[0].tool_calls?.[0].id).toBe("call_456");
      expect(result[0].tool_calls?.[0].function.name).toBe("tree");
      expect(result[0].tool_calls?.[0].function.arguments).toBe(
        '{"path": "/"}',
      );
      expect(result[0].content).toBe("Let me search and check the files.");
      expect(result[0].server_executed_tools).toHaveLength(1);
      expect(result[0].server_executed_tools?.[0].function.name).toBe(
        "web_search",
      );
      expect(result[0].server_executed_tools?.[0].function.arguments).toBe(
        '{"query": "test search"}',
      );
    }
  });
});
