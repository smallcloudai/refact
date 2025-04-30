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
      content: " what is this for?\n",
      role: "user",
    };

    const messages: ChatMessages = [
      { role: "user", content: "Hello" },
      {
        role: "assistant",
        content: "Hi",
        tool_calls: [
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
        role: "tool",
        content: {
          tool_call_id: "call_6qxVYwV6MTcazl1Fy5pRlImi",
          content: "stuff",
        },
      },
      {
        role: "context_file",
        content: [
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
        role: "assistant",
        content: "test response",
      },
      {
        role: "user",
        content:
          "@file /Users/marc/Projects/refact-chat-js/src/__fixtures__/chat_diff.ts what is this for?\n",
      },
      {
        role: "context_file",
        content: [
          {
            file_content: "test content",
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
      { role: message.role, content: message.content },
    ];

    expect(result).toEqual(expected);
  });

  test("it should put plain text before a user message at the end of the array", () => {
    const userMessage: UserMessage = {
      role: "user",
      content: "Hello",
    };

    const sentMessages = [userMessage];

    const updatedUserMessage: UserMessage = {
      role: "user",
      content: "hi",
    };

    const userMessageResponse: UserMessageResponse = {
      ...updatedUserMessage,
      id: "user message",
    };

    const plainTextMessage: PlainTextMessage = {
      role: "plain_text",
      content: "test",
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
        role: "user",
        content: "Hello\n",
        checkpoints: [
          {
            workspace_folder: "/Users/marc/Projects/refact",
            commit_hash: "9ff4c36e0f41f1a599a9ff203f00389c6c479b88",
          },
        ],
        compression_strength: "absent",
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: "Hello",
              role: "assistant",
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: "!",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " How",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " can",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " I",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " assist",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " you",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: " today",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              // refusal: null,
              content: "?",
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        // citations: null,
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: "stop",
            index: 0,
            delta: {
              // provider_specific_fields: null,
              content: null,
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              content: null,
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
      },
      {
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              content: null,
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        usage: {
          completion_tokens: 11,
          prompt_tokens: 2818,
          total_tokens: 2829,
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
        id: "chatcmpl-0f045407-74a8-4278-ace1-8e7c96b2c50e",
        created: 1745945724.54118,
        model: "gpt-4o",
        // object: "chat.completion.chunk",
        // system_fingerprint: "fp_f5bdcc3276",
        choices: [
          {
            finish_reason: null,
            index: 0,
            delta: {
              // provider_specific_fields: null,
              content: null,
              role: null,
              // function_call: null,
              tool_calls: null,
              // audio: null,
            },
            // logprobs: null,
          },
        ],
        // provider_specific_fields: null,
        // stream_options: { include_usage: true },
        usage: {
          completion_tokens: 11,
          prompt_tokens: 2818,
          total_tokens: 2829,
          completion_tokens_details: {
            accepted_prediction_tokens: 0,
            audio_tokens: 0,
            reasoning_tokens: 0,
            rejected_prediction_tokens: 0,
          },
          prompt_tokens_details: { audio_tokens: 0, cached_tokens: 0 },
        },
        pp1000t_prompt: 2500,
        pp1000t_generated: 10000,
        pp1000t_cache_creation: 0,
        pp1000t_cache_read: 1250,
        metering_prompt_tokens_n: 2818,
        metering_generated_tokens_n: 11,
        metering_cache_creation_tokens_n: 0,
        metering_cache_read_tokens_n: 0,
        metering_balance: -2688820,
      },
      {
        choices: [
          {
            index: 0,
            delta: { role: "assistant", content: "", tool_calls: null },
            finish_reason: "stop",
          },
        ],
        // object: "chat.completion.chunk",
        created: 1745945724.54118,
        model: "gpt-4o",
        id: "", // missing irl
      },
    ];

    const result = chunks.reduce<ChatMessages>((acc, cur) => {
      return formatChatResponse(acc, cur);
    }, []);

    expect(result).toEqual([
      {
        checkpoints: [
          {
            commit_hash: "9ff4c36e0f41f1a599a9ff203f00389c6c479b88",
            workspace_folder: "/Users/marc/Projects/refact",
          },
        ],
        compression_strength: "absent",
        content: "Hello\n",
        role: "user",
      },
      {
        content: "Hello! How can I assist you today?",
        finish_reason: "stop",
        metering_balance: -2688820,
        metering_cache_creation_tokens_n: 0,
        metering_cache_read_tokens_n: 0,
        metering_prompt_tokens_n: 2818,
        pp1000t_cache_creation: 0,
        pp1000t_cache_read: 1250,
        pp1000t_generated: 10000,
        pp1000t_prompt: 2500,
        reasoning_content: "",
        role: "assistant",
        thinking_blocks: undefined,
        tool_calls: undefined,
        usage: {
          completion_tokens: 11,
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
          total_tokens: 2829,
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

describe.skip("cache", () => {
  // test("loading the cache correctly", () => {
  //   const initialState = createInitialState();
  //   const chat1id = uuidv4();
  //   const chat2id = uuidv4();
  //   function create_restore_chat(
  //     fromId: string,
  //     toId: string,
  //     message: string,
  //   ) {
  //     return {
  //       type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
  //       payload: {
  //         id: fromId,
  //         chat: {
  //           messages: [["user", message]],
  //           model: "gpt-3.5-turbo",
  //           id: toId,
  //         },
  //       },
  //     };
  //   }
  //   function create_chat_response(id: string, message: string) {
  //     return {
  //       type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
  //       payload: {
  //         id,
  //         choices: [
  //           {
  //             delta: {
  //               content: message,
  //               role: "assistant",
  //             },
  //             finish_reason: null,
  //             index: 0,
  //           },
  //         ],
  //         created: 1710777171.188,
  //         model: "gpt-3.5-turbo",
  //       },
  //     };
  //   }
  //   const actions: ActionToChat[] = [
  //     create_restore_chat(initialState.chat.id, chat1id, "Hello"),
  //     create_chat_response(chat1id, "test"),
  //     create_restore_chat(chat1id, chat2id, "Goodbye"),
  //     create_chat_response(chat1id, " response"),
  //     create_restore_chat(chat2id, chat1id, "Test"),
  //   ];
  //   expect(() => {
  //     const reduce = reducer(window.postMessage);
  //     let state = initialState;
  //     for (const action of actions) {
  //       state = reduce(state, action);
  //     }
  //     expect(state.chat.messages).toEqual([
  //       ["user", "Hello"],
  //       ["assistant", "test response", undefined],
  //     ]);
  //   }).not.toThrow();
  // });
});
