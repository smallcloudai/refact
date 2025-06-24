import { describe, expect, test, vi } from "vitest";
import {
  ChatMessages,
  ChatResponse,
  PlainTextMessage,
  PlainTextResponse,
  UserMessage,
  UserMessageResponse,
  type ToolCall,
} from "../../../services/refact";
import { mergeToolCalls, formatChatResponse, consumeStream } from "./utils";

describe("formatChatResponse", () => {
  test("it should replace the last user message", () => {
    const message: UserMessageResponse = {
      id: "test",
      ftm_content: " what is this for?\n",
      ftm_role: "user",
    };

    const messages: ChatMessages = [
      { ftm_role: "user", ftm_content: "Hello" },
      {
        ftm_role: "assistant",
        ftm_content: "Hi",
        ftm_tool_calls: [
          {
            function: {
              arguments:
                '{"problem_statement":"What is the difference between the Toad and Frog classes?"}',
              name: "locate",
            },
            id: "call_6qxVYwV6MTcazl1Fy5pRlImi",
            index: 0,
            type: "function",
          },
        ],
      },
      {
        ftm_role: "tool",
        ftm_call_id: "call_6qxVYwV6MTcazl1Fy5pRlImi",
        ftm_content: "stuff",
      },
      {
        ftm_role: "context_file",
        ftm_content: [
          {
            file_content: "stuff",
            file_name: "refact-chat-js/src/services/refact/chat.ts",
            line1: 1,
            line2: 85,
            usefulness: 0,
          },
        ],
      },
      {
        ftm_role: "assistant",
        ftm_content: "test response",
      },
      {
        ftm_role: "user",
        ftm_content:
          "@file /Users/marc/Projects/refact-chat-js/src/__fixtures__/chat_diff.ts what is this for?\n",
      },
      {
        ftm_role: "context_file",
        ftm_content: [
          {
            file_content: "test ftm_content",
            file_name: "refact-chat-js/src/__fixtures__/chat_diff.ts",
            line1: 1,
            line2: 30,
            usefulness: 0,
          },
        ],
      },
    ];

    const result = formatChatResponse(messages, message);

    const expected = [
      ...messages.slice(0, 5),
      ...messages.slice(6),
      { ftm_role: message.ftm_role, ftm_content: message.ftm_content },
    ];

    expect(result).toEqual(expected);
  });

  test("it should put plain text before a user message at the end of the array", () => {
    const userMessage: UserMessage = {
      ftm_role: "user",
      ftm_content: "Hello",
    };

    const sentMessages = [userMessage];

    const updatedUserMessage: UserMessage = {
      ftm_role: "user",
      ftm_content: "hi",
    };

    const userMessageResponse: UserMessageResponse = {
      ...updatedUserMessage,
      id: "user message",
    };

    const plainTextMessage: PlainTextMessage = {
      ftm_role: "plain_text",
      ftm_content: "test",
    };

    const plainTextResponse: PlainTextResponse = {
      ...plainTextMessage,
      tool_call_id: "toolCallId",
    };

    const response = [plainTextResponse, userMessageResponse];

    const result = response.reduce<ChatMessages>((messages, message) => {
      return formatChatResponse(messages, message);
    }, sentMessages);

    const expected = [plainTextMessage, updatedUserMessage];

    expect(result).toEqual(expected);
  });

  test("price with message", () => {
    const chunks: ChatResponse[] = [
      {
        id: "",
        ftm_role: "user",
        ftm_content: "hello\n",
        checkpoints: [
          {
            workspace_folder: "/refact",
            commit_hash: "6710babc75beb5198be8a7a2b4ba6c095afa2158",
          },
        ],
        compression_strength: "absent",
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "Hello",
              ftm_role: "assistant",
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "!",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " How",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " can",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " I",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " assist",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " you",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " with",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " your",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " project",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " today",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "?",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: "stop",
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
        usage: {
          completion_tokens: 14,
          prompt_tokens: 2818,
          total_tokens: 2832,
          completion_tokens_details: {
            accepted_prediction_tokens: 0,
            audio_tokens: 0,
            reasoning_tokens: 0,
            rejected_prediction_tokens: 0,
          },
          prompt_tokens_details: { audio_tokens: 0, cached_tokens: 0 },
        },
      },
      {
        id: "chatcmpl-d103cc09-5306-43d3-9fb3-609e5e61948a",
        created: 1746094949.359174,
        model: "gpt-4.1",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
        usage: {
          completion_tokens: 14,
          prompt_tokens: 2818,
          total_tokens: 2832,
          completion_tokens_details: {
            accepted_prediction_tokens: 0,
            audio_tokens: 0,
            reasoning_tokens: 0,
            rejected_prediction_tokens: 0,
          },
          prompt_tokens_details: { audio_tokens: 0, cached_tokens: 0 },
        },
        metering_coins_prompt: 5.636,
        metering_coins_generated: 0.112,
        metering_coins_cache_creation: 0.0,
        metering_coins_cache_read: 0.0,
        metering_prompt_tokens_n: 2818,
        metering_generated_tokens_n: 14,
        metering_cache_creation_tokens_n: 0,
        metering_cache_read_tokens_n: 0,
        metering_balance: 1085,
        refact_agent_request_available: null,
        refact_agent_max_request_num: 40,
      },
      {
        id: "",
        choices: [
          {
            index: 0,
            delta: { ftm_role: "assistant", ftm_content: "", tool_calls: null },
            finish_reason: "stop",
          },
        ],
        created: 1746094949.359174,
        model: "gpt-4.1",
      },
    ];

    const result = chunks.reduce<ChatMessages>((acc, cur) => {
      return formatChatResponse(acc, cur);
    }, []);

    expect(result).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "6710babc75beb5198be8a7a2b4ba6c095afa2158",
            workspace_folder: "/refact",
          },
        ],
        compression_strength: "absent",
        ftm_content: "hello\n",
        ftm_role: "user",
      },
      {
        ftm_content: "Hello! How can I assist you with your project today?",
        finish_reason: "stop",
        metering_balance: 1085,
        metering_cache_creation_tokens_n: 0,
        metering_cache_read_tokens_n: 0,
        metering_coins_cache_creation: 0,
        metering_coins_cache_read: 0,
        metering_coins_generated: 0.112,
        metering_coins_prompt: 5.636,
        metering_prompt_tokens_n: 2818,
        metering_generated_tokens_n: 14,
        reasoning_content: "",
        ftm_role: "assistant",
        thinking_blocks: undefined,
        tool_calls: undefined,
        usage: {
          completion_tokens: 14,
          completion_tokens_details: {
            accepted_prediction_tokens: 0,
            audio_tokens: 0,
            reasoning_tokens: 0,
            rejected_prediction_tokens: 0,
          },
          prompt_tokens: 2818,
          prompt_tokens_details: {
            audio_tokens: 0,
            cached_tokens: 0,
          },
          total_tokens: 2832,
        },
      },
    ]);
  });

  test("byok usage", () => {
    const chunks: ChatResponse[] = [
      {
        id: "",
        ftm_role: "user",
        ftm_content: "call tree and then do nothing\n",
        checkpoints: [
          {
            workspace_folder: "/someplace",
            commit_hash: "d7fd24f70133348f01a80f6f9a54628e2ee56777",
          },
        ],
        compression_strength: "absent",
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "I'll call",
              ftm_role: "assistant",
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " the `tree` function to show the project structure",
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: " and then do nothing else as requested.",
              ftm_role: null,

              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "",
              ftm_role: "assistant",

              tool_calls: [
                {
                  id: "toolu_01SZSQHfY6jRi4TSd9HTRy6e",
                  function: {
                    arguments: "",
                    name: "tree",
                  },
                  type: "function",
                  index: 0,
                },
              ],
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "",
              ftm_role: "assistant",

              tool_calls: [
                // odd that some of these are null?
                // {
                //   id: null,
                //   function: {
                //     arguments: "",
                //     name: null,
                //   },
                //   type: "function",
                //   index: 0,
                // },
              ],
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: "",
              ftm_role: "assistant",

              tool_calls: [
                // {
                //   id: null,
                //   function: {
                //     arguments: "{}",
                //     name: null,
                //   },
                //   type: "function",
                //   index: 0,
                // },
              ],
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: "tool_calls",
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,

              tool_calls: null,
            },
          },
        ],

        usage: {
          completion_tokens: 56,
          prompt_tokens: 3,
          total_tokens: 59,
          completion_tokens_details: {
            accepted_prediction_tokens: null,
            audio_tokens: null,
            reasoning_tokens: 0,
            rejected_prediction_tokens: null,
          },
          prompt_tokens_details: {
            audio_tokens: null,
            cached_tokens: 0,
          },
          cache_creation_input_tokens: 9170,
          cache_read_input_tokens: 0,
        },
      },
      {
        id: "chatcmpl-db1e8dbd-5170-4a35-bc62-ae5aa6f46fa4",
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",

        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              ftm_content: null,
              ftm_role: null,
              tool_calls: null,
            },
          },
        ],
        usage: {
          completion_tokens: 56,
          prompt_tokens: 3,
          total_tokens: 59,
          completion_tokens_details: {
            accepted_prediction_tokens: null,
            audio_tokens: null,
            reasoning_tokens: 0,
            rejected_prediction_tokens: null,
          },
          prompt_tokens_details: {
            audio_tokens: null,
            cached_tokens: 0,
          },
          cache_creation_input_tokens: 9170,
          cache_read_input_tokens: 0,
        },
        metering_coins_prompt: 0.009,
        metering_coins_generated: 0.84,
        metering_coins_cache_creation: 34.3875,
        metering_coins_cache_read: 0.0,
        metering_prompt_tokens_n: 3,
        metering_generated_tokens_n: 56,
        metering_cache_creation_tokens_n: 9170,
        metering_cache_read_tokens_n: 0,
        metering_balance: 952433,
        refact_agent_request_available: null,
        refact_agent_max_request_num: 400,
      },
      {
        id: "",
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
              ftm_content: "",
              tool_calls: null,
            },
            finish_reason: "stop",
          },
        ],
        created: 1746115727.9020996,
        model: "claude-3-7-sonnet",
      },
    ];

    const results = chunks.reduce<ChatMessages>(
      (acc, cur) => formatChatResponse(acc, cur),
      [],
    );

    expect(results).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "d7fd24f70133348f01a80f6f9a54628e2ee56777",
            workspace_folder: "/someplace",
          },
        ],
        compression_strength: "absent",
        ftm_content: "call tree and then do nothing\n",
        ftm_role: "user",
      },
      {
        ftm_content:
          "I'll call the `tree` function to show the project structure and then do nothing else as requested.",
        finish_reason: "stop",
        metering_balance: 952433,
        metering_cache_creation_tokens_n: 9170,
        metering_cache_read_tokens_n: 0,
        metering_coins_cache_creation: 34.3875,
        metering_coins_cache_read: 0,
        metering_coins_generated: 0.84,
        metering_coins_prompt: 0.009,
        metering_prompt_tokens_n: 3,
        metering_generated_tokens_n: 56,
        reasoning_content: "",
        ftm_role: "assistant",
        thinking_blocks: undefined,
        tool_calls: [
          {
            function: {
              arguments: "",
              name: "tree",
            },
            id: "toolu_01SZSQHfY6jRi4TSd9HTRy6e",
            index: 0,
            type: "function",
          },
        ],
        usage: {
          cache_creation_input_tokens: 9170,
          cache_read_input_tokens: 0,
          completion_tokens: 56,
          completion_tokens_details: {
            accepted_prediction_tokens: null,
            audio_tokens: null,
            reasoning_tokens: 0,
            rejected_prediction_tokens: null,
          },
          prompt_tokens: 3,
          prompt_tokens_details: {
            audio_tokens: null,
            cached_tokens: 0,
          },
          total_tokens: 59,
        },
      },
    ]);
  });

  test("byok short usage", () => {
    const chunks: ChatResponse[] = [
      {
        id: "",
        ftm_role: "user",
        ftm_content: "please tell me a joke, don't call any tools\n",
        checkpoints: [
          {
            workspace_folder:
              "/home/andrii-lashchov/Desktop/work/refact/refact-agent/engine",
            commit_hash: "b71c8387f951b81a1b9cd388f3d46c94eb302ebe",
          },
        ],
        compression_strength: "absent",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "I'",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "d tell you a joke about UDP, but you",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " might not get it.\n\nWait",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: ", here's another one:",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " Why do programmers prefer dark mode?",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " Because light attracts bugs!",
            },
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
      {
        id: "msg_01SrL8iCZWJGWhYF2obVNXeV",
        choices: [
          {
            index: 0,
            delta: {},
            finish_reason: "stop",
          },
        ],
        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
        usage: {
          completion_tokens: 41,
          prompt_tokens: 9359,
          total_tokens: 9400,
        },
      },
      {
        id: "",
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
              ftm_content: "",
              tool_calls: null,
            },
            finish_reason: "stop",
          },
        ],

        created: 1746117659.9634643,
        model: "claude-3-7-sonnet-latest",
      },
    ];

    const result = chunks.reduce<ChatMessages>(
      (messages, chunk) => formatChatResponse(messages, chunk),
      [],
    );

    expect(result).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "b71c8387f951b81a1b9cd388f3d46c94eb302ebe",
            workspace_folder:
              "/home/andrii-lashchov/Desktop/work/refact/refact-agent/engine",
          },
        ],
        compression_strength: "absent",
        ftm_content: "please tell me a joke, don't call any tools\n",
        ftm_role: "user",
      },
      {
        ftm_content:
          "I'd tell you a joke about UDP, but you might not get it.\n\nWait, here's another one: Why do programmers prefer dark mode? Because light attracts bugs!",
        finish_reason: "stop",
        metering_balance: undefined,
        metering_cache_creation_tokens_n: undefined,
        metering_cache_read_tokens_n: undefined,
        metering_coins_cache_creation: undefined,
        metering_coins_cache_read: undefined,
        metering_coins_generated: undefined,
        metering_coins_prompt: undefined,
        metering_prompt_tokens_n: undefined,
        reasoning_content: "",
        ftm_role: "assistant",
        thinking_blocks: undefined,
        tool_calls: undefined,
        usage: {
          completion_tokens: 41,
          prompt_tokens: 9359,
          total_tokens: 9400,
        },
      },
    ]);
  });

  test("gemini", () => {
    const chunks: ChatResponse[] = [
      {
        id: "",
        ftm_role: "user",
        ftm_content: "call tree\n",
        checkpoints: [
          {
            workspace_folder: "/emergency_frog_situation",
            commit_hash: "9592d97a746d392d180491bd5a44339d83f1c19c",
          },
        ],
        compression_strength: "absent",
      },
      {
        choices: [
          {
            delta: {
              ftm_content: "Okay, I will",
              ftm_role: "assistant",
            },
            index: 0,
          },
        ],
        created: 1746186404.4522197,
        model: "gemini-2.5-pro-exp-03-25",
        id: "",
        usage: {
          completion_tokens: 4,
          prompt_tokens: 3547,
          total_tokens: 3577,
        },
      },
      {
        choices: [
          {
            delta: {
              ftm_content:
                " call the `tree()` tool to show the project structure.",
              ftm_role: "assistant",
            },
            index: 0,
          },
        ],
        created: 1746186404.4522197,
        model: "gemini-2.5-pro-exp-03-25",
        id: "",
        usage: {
          completion_tokens: 16,
          prompt_tokens: 3547,
          total_tokens: 3601,
        },
      },
      {
        choices: [
          {
            delta: {
              ftm_role: "assistant",
              tool_calls: [
                {
                  function: {
                    arguments: "{}",
                    name: "tree",
                  },
                  id: "call_247e2a7b080d44fe83a655fd18d17277",
                  type: "function",
                  index: 0,
                },
              ],
            },
            finish_reason: "tool_calls",
            index: 0,
          },
        ],
        created: 1746186404.4522197,
        model: "gemini-2.5-pro-exp-03-25",
        usage: {
          completion_tokens: 24,
          prompt_tokens: 3547,
          total_tokens: 3604,
        },
      },
      {
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
              ftm_content: "",
              tool_calls: null,
            },
            finish_reason: "stop",
          },
        ],
        created: 1746186404.4522197,
        model: "gemini-2.5-pro-exp-03-25",
      },
    ];

    const result = chunks.reduce<ChatMessages>(
      (acc, cur) => formatChatResponse(acc, cur),
      [],
    );

    expect(result).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "9592d97a746d392d180491bd5a44339d83f1c19c",
            workspace_folder: "/emergency_frog_situation",
          },
        ],
        compression_strength: "absent",
        ftm_content: "call tree\n",
        ftm_role: "user",
      },
      {
        ftm_content:
          "Okay, I will call the `tree()` tool to show the project structure.",
        finish_reason: "stop",
        metering_balance: undefined,
        metering_cache_creation_tokens_n: undefined,
        metering_cache_read_tokens_n: undefined,
        metering_coins_cache_creation: undefined,
        metering_coins_cache_read: undefined,
        metering_coins_generated: undefined,
        metering_coins_prompt: undefined,
        metering_prompt_tokens_n: undefined,
        reasoning_content: "",
        ftm_role: "assistant",
        thinking_blocks: undefined,
        tool_calls: [
          {
            function: {
              arguments: "{}",
              name: "tree",
            },
            id: "call_247e2a7b080d44fe83a655fd18d17277",
            index: 0,
            type: "function",
          },
        ],
        usage: {
          completion_tokens: 24,
          prompt_tokens: 3547,
          total_tokens: 3604,
        },
      },
    ]);
  });

  test("byok openai usage", () => {
    const chunks: ChatResponse[] = [
      {
        id: "",
        ftm_role: "user",
        ftm_content: "hello\n",
        checkpoints: [
          {
            workspace_folder: "/Users/marc/Projects/refact",
            commit_hash: "5365c0e1efde9a8a4b9be199ea8cd47e4cc5acfd",
          },
        ],
        compression_strength: "absent",
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
              ftm_content: "",
              // refusal: null,
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "Hello",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "!",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " I'm",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " Ref",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "act",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " Agent",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: ",",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " your",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " coding",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " assistant",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: ".",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " How",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " can",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " I",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " help",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " you",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: " today",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {
              ftm_content: "?",
            },
            finish_reason: null,
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [
          {
            index: 0,
            delta: {},
            finish_reason: "stop",
          },
        ],
        usage: null,
      },
      {
        id: "chatcmpl-BUBWQDOHxOWUxzDW2DxvUR462yMpT",
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
        // service_tier: "default",
        // system_fingerprint: "fp_8810992130",
        choices: [],
        usage: {
          prompt_tokens: 2876,
          completion_tokens: 222,
          total_tokens: 3098,
          prompt_tokens_details: {
            cached_tokens: 2688,
            audio_tokens: 0,
          },
          completion_tokens_details: {
            reasoning_tokens: 192,
            audio_tokens: 0,
            accepted_prediction_tokens: 0,
            rejected_prediction_tokens: 0,
          },
        },
      },
      {
        choices: [
          {
            index: 0,
            delta: {
              ftm_role: "assistant",
              ftm_content: "",
              tool_calls: null,
            },
            finish_reason: "stop",
          },
        ],
        // object: "chat.completion.chunk",
        created: 1746533829.888066,
        model: "o3-mini",
      },
    ];

    const result = chunks.reduce<ChatMessages>(
      (acc, cur) => formatChatResponse(acc, cur),
      [],
    );

    expect(result).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "5365c0e1efde9a8a4b9be199ea8cd47e4cc5acfd",
            workspace_folder: "/Users/marc/Projects/refact",
          },
        ],
        compression_strength: "absent",
        ftm_content: "hello\n",
        ftm_role: "user",
      },
      {
        ftm_content:
          "Hello! I'm Refact Agent, your coding assistant. How can I help you today?",
        finish_reason: "stop",
        metering_balance: undefined,
        metering_cache_creation_tokens_n: undefined,
        metering_cache_read_tokens_n: undefined,
        metering_coins_cache_creation: undefined,
        metering_coins_cache_read: undefined,
        metering_coins_generated: undefined,
        metering_coins_prompt: undefined,
        metering_prompt_tokens_n: undefined,
        reasoning_content: "",
        ftm_role: "assistant",
        thinking_blocks: undefined,
        tool_calls: undefined,
        usage: {
          prompt_tokens: 2876,
          completion_tokens: 222,
          total_tokens: 3098,
          prompt_tokens_details: {
            cached_tokens: 2688,
            audio_tokens: 0,
          },
          completion_tokens_details: {
            reasoning_tokens: 192,
            audio_tokens: 0,
            accepted_prediction_tokens: 0,
            rejected_prediction_tokens: 0,
          },
        },
      },
    ]);
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

function stringToUint8Array(str: string): Uint8Array {
  const encoder = new TextEncoder();
  return encoder.encode(str);
}

describe("consumeStream", () => {
  test("it should handle split packets", async () => {
    const packet1 = stringToUint8Array('data: {"key": "test"}\n\n');
    const packet2 = stringToUint8Array('data: {"key":');
    const packet3 = stringToUint8Array('"value"}\n\n');

    const reader = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(packet1);
        controller.enqueue(packet2);
        controller.enqueue(packet3);
        controller.close();
      },
    }).getReader();

    const onAbort = vi.fn();
    const onChunk = vi.fn();
    const abort = new AbortController();

    await consumeStream(reader, abort.signal, onAbort, onChunk);

    expect(onAbort).not.toBeCalled();
    expect(onChunk).toBeCalledWith({ key: "test" });
    expect(onChunk).toBeCalledWith({ key: "value" });
  });

  test("it only splits at \\n\\n", async () => {
    const packet1 = stringToUint8Array(
      'data: {"content":"```py\\nprint(\\"hello\\")\\n\\n',
    );
    const packet2 = stringToUint8Array('```\\n"}\n\n');

    const reader = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(packet1);
        controller.enqueue(packet2);
        controller.close();
      },
    }).getReader();

    const onAbort = vi.fn();
    const onChunk = vi.fn();
    const abort = new AbortController();

    await consumeStream(reader, abort.signal, onAbort, onChunk);

    expect(onAbort).not.toBeCalled();

    expect(onChunk).toHaveBeenCalledWith({
      content: '```py\nprint("hello")\n\n```\n',
    });
  });
});
