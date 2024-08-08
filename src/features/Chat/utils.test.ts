import { describe, expect, test } from "vitest";
// import { v4 as uuidv4 } from "uuid";
// import { reducer, createInitialState } from ".";
import {
  // EVENT_NAMES_TO_CHAT,
  ToolCall,
  // ResponseToChat,
  // ActionToChat,
} from "../../events";
import { mergeToolCalls } from "./utils";

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
