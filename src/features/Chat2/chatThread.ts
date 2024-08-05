import { createReducer, createAction } from "@reduxjs/toolkit";
import { v4 as uuidv4 } from "uuid";
import {
  LspChatMessage,
  ToolCommand,
  // isChatUserMessageResponse,
} from "../../events";
// TODO: update this type
import { type ChatResponse } from "../../events";
import {
  useAppDispatch,
  createAppAsyncThunk,
  useAppSelector,
  useGetCapsQuery,
  useGetToolsQuery,
} from "../../app/hooks";
import { parseOrElse } from "../../utils";
import { useCallback } from "react";

export type ChatThread = {
  id: string;
  messages: LspChatMessage[];
  model: string;
  title?: string;
  attach_file?: boolean;
  createdAt?: string;
  lastUpdated?: string;
};

export type Chat = {
  streaming: boolean;
  thread: ChatThread;
  error: null | string;
  prevent_send: boolean;
  previous_message_length: number;
  waiting_for_response: boolean;
  cache: Record<string, ChatThread>;
};

const createChatThread = (): ChatThread => {
  const chat: ChatThread = {
    id: uuidv4(),
    messages: [],
    title: "",
    model: "",
  };
  return chat;
};

const createInitialState = (): Chat => {
  return {
    streaming: false,
    thread: createChatThread(),
    error: null,
    prevent_send: false,
    previous_message_length: 0,
    waiting_for_response: false,
    cache: {},
  };
};

const initialState = createInitialState();

type PayloadWIthId = { id: string };
const newChatAction = createAction<PayloadWIthId>("chatThread/new");

// const doneStreaming = createAction<PayloadWIthId>("chatThread/doneStreaming");

const chatResponse = createAction<PayloadWIthId & ChatResponse>(
  "chatThread/response",
);

const chatAskedQuestion = createAction<PayloadWIthId>("chatThread/askQuestion");

const backUpMessages = createAction<
  PayloadWIthId & { messages: ChatThread["messages"] }
>("chatThread/backUpMessages");

const chatError = createAction<PayloadWIthId & { message: string }>(
  "chatThread/error",
);

const doneStreaming = createAction<PayloadWIthId>("chatThread/doneStreaming");

// ask question

export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(newChatAction, (state, action) => {
    if (state.thread.id === action.payload.id) {
      state = createInitialState();
    }
  });

  builder.addCase(chatResponse, (state, action) => {
    // TODO: handle chache
    if (state.thread.id !== action.payload.id) return state;
    // const hasUserMessage = isChatUserMessageResponse(action.payload);

    // const current = hasUserMessage
    //   ? state.thread.messages.slice(0, state.previous_message_length)
    //   : state.thread.messages;
    // const messages = formatChatResponse(current, action.payload);

    // return {
    //   ...state,
    //   // files_in_preview: [],
    //   waiting_for_response: false,
    //   streaming: true,
    //   previous_message_length: messages.length,
    //   chat: {
    //     ...state.chat,
    //     messages,
    //     // applied_diffs: {},
    //   },
    // };

    state.waiting_for_response = false;
  });

  builder.addCase(backUpMessages, (state, action) => {
    // TODO: should it also save to history?
    state.error = null;
    // state.previous_message_length = state.thread.messages.length;
    state.previous_message_length = action.payload.messages.length - 1;
    state.thread.messages = action.payload.messages;
  });

  builder.addCase(chatError, (state, action) => {
    state.streaming = false;
    state.prevent_send = true;
    state.waiting_for_response = false;
    state.error = action.payload.message;
  });

  builder.addCase(doneStreaming, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.streaming = false;
  });

  builder.addCase(chatAskedQuestion, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.waiting_for_response = true;
    state.streaming = true;
  });
});

// this will need the chat id and tools

// export function formatChatResponse(
//     messages: ChatMessages,
//     response: ChatResponse,
//   ): LspChatMessage[] {
//     if (isChatUserMessageResponse(response)) {
//       if (response.role === "context_file") {
//         const content = parseOrElse<ChatContextFile[]>(response.content, []);
//         return [...messages, {...response, content}];
//       } else if (response.role === "context_memory") {
//         const content = parseOrElse<ContextMemory[]>(response.content, []);
//         return [...messages, [response.role, content]];
//       }

//       return [...messages, [response.role, response.content]];
//     }

//     if (isToolResponse(response)) {
//       const { tool_call_id, content, finish_reason } = response;
//       const toolResult: ToolResult = { tool_call_id, content, finish_reason };
//       return [...messages, [response.role, toolResult]];
//     }

//     if (isDiffResponse(response)) {
//       const content = parseOrElse<DiffChunk[]>(response.content, []);
//       return [...messages, [response.role, content, response.tool_call_id]];
//     }

//     if (isPlainTextResponse(response)) {
//       return [...messages, [response.role, response.content]];
//     }

//     if (!isChatResponseChoice(response)) {
//       // console.log("Not a good response");
//       // console.log(response);
//       return messages;
//     }

//     return response.choices.reduce<ChatMessages>((acc, cur) => {
//       if (isChatContextFileDelta(cur.delta)) {
//         return acc.concat([[cur.delta.role, cur.delta.content]]);
//       }

//       if (
//         acc.length === 0 &&
//         "content" in cur.delta &&
//         typeof cur.delta.content === "string" &&
//         cur.delta.role
//       ) {
//         if (cur.delta.role === "assistant") {
//           return acc.concat([
//             [cur.delta.role, cur.delta.content, cur.delta.tool_calls],
//           ]);
//         }
//         // TODO: narrow this
//         const message = [cur.delta.role, cur.delta.content] as ChatMessage;
//         return acc.concat([message]);
//       }

//       const lastMessage = acc[acc.length - 1];

//       if (isToolCallDelta(cur.delta)) {
//         if (!isAssistantMessage(lastMessage)) {
//           return acc.concat([
//             ["assistant", cur.delta.content ?? "", cur.delta.tool_calls],
//           ]);
//         }

//         const last = acc.slice(0, -1);
//         const collectedCalls = lastMessage[2] ?? [];
//         const calls = mergeToolCalls(collectedCalls, cur.delta.tool_calls);
//         const content = cur.delta.content;
//         const message = content ? lastMessage[1] + content : lastMessage[1];

//         return last.concat([["assistant", message, calls]]);
//       }

//       if (
//         isAssistantMessage(lastMessage) &&
//         isAssistantDelta(cur.delta) &&
//         typeof cur.delta.content === "string"
//       ) {
//         const last = acc.slice(0, -1);
//         const currentMessage = lastMessage[1] ?? "";
//         const toolCalls = lastMessage[2];
//         return last.concat([
//           ["assistant", currentMessage + cur.delta.content, toolCalls],
//         ]);
//       } else if (
//         isAssistantDelta(cur.delta) &&
//         typeof cur.delta.content === "string"
//       ) {
//         return acc.concat([["assistant", cur.delta.content]]);
//       } else if (cur.delta.role === "assistant") {
//         // empty message from JB
//         return acc;
//       }

//       if (cur.delta.role === null || cur.finish_reason !== null) {
//         return acc;
//       }

//       // console.log("Fall though");
//       // console.log({ cur, lastMessage });

//       return acc;
//     }, messages);
//   }

const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: LspChatMessage[];
    chatId: string;
    tools: ToolCommand[] | null;
  }
>("chatThread/sendChat", ({ messages, chatId, tools }, thunkAPI) => {
  const state = thunkAPI.getState();
  sendChat({
    messages,
    model: state.chat.thread.model,
    tools,
    stream: true,
    abortSignal: thunkAPI.signal,
    // should be passes as well
    chatId,
  })
    .then((response) => {
      if (!response.ok) {
        return Promise.reject(new Error(response.statusText));
      }

      const decoder = new TextDecoder();
      const reader = response.body?.getReader();
      if (!reader) return;

      return reader.read().then(function pump({ done, value }): Promise<void> {
        if (done) return Promise.resolve();
        if (thunkAPI.signal.aborted) return Promise.resolve();

        const streamAsString = decoder.decode(value);

        const deltas = streamAsString
          .split("\n\n")
          .filter((str) => str.length > 0);
        if (deltas.length === 0) return Promise.resolve();

        // could be improved
        for (const delta of deltas) {
          if (!delta.startsWith("data: ")) {
            // eslint-disable-next-line no-console
            console.log("Unexpected data in streaming buf: " + delta);
            continue;
          }

          const maybeJsonString = delta.substring(6);

          if (maybeJsonString === "[DONE]") return Promise.resolve();

          if (maybeJsonString === "[ERROR]") {
            // check for error details
            const errorMessage = "error from lsp";
            const error = new Error(errorMessage);

            return Promise.reject(error);
          }

          // TODO: add better type chekcing
          const json = parseOrElse<Record<string, unknown>>(
            maybeJsonString,
            {},
          );

          if ("detail" in json) {
            const errorMessage: string =
              typeof json.detail === "string"
                ? json.detail
                : JSON.stringify(json.detail);
            const error = new Error(errorMessage);

            // eslint-disable-next-line no-console
            console.error(error);
            return Promise.reject(error);
          }

          // TODO: type check this. also some models create a new id :/
          thunkAPI.dispatch(
            chatResponse({ ...(json as ChatResponse), id: chatId }),
          );
        }

        return reader.read().then(pump);
      });
    })
    .catch((err: Error) => {
      return thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
    })
    .finally(() => {
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
    });
});

export const useSendChatRequest = () => {
  const chatId = useAppSelector((state) => state.chat.thread.id);
  const dispatch = useAppDispatch();
  const capsRequest = useGetCapsQuery(undefined);
  const toolsRequest = useGetToolsQuery(!!capsRequest.data);

  const currentMesssages = useAppSelector(
    (state) => state.chat.thread.messages,
  );
  const submit = useCallback(
    (question: string) => {
      const tools = toolsRequest.data ?? null;
      const message: LspChatMessage = { role: "user", content: question };

      const messages = currentMesssages.concat(message);
      dispatch(backUpMessages({ id: chatId, messages }));
      dispatch(chatAskedQuestion({ id: chatId }));

      const action = chatAskQuestionThunk({
        messages,
        tools,
        chatId,
      });

      return dispatch(action);
    },
    [chatId, currentMesssages, dispatch, toolsRequest.data],
  );

  // TODO: ask imidiantly for rconsole and context menu questions.

  return {
    submit,
  };
};

// Streaming should be handled elsewhere

type StreamArgs =
  | {
      stream: true;
      abortSignal: AbortSignal;
    }
  | { stream: false; abortSignal?: undefined | AbortSignal };

type SendChatArgs = {
  messages: LspChatMessage[];
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  tools: ToolCommand[] | null;
} & StreamArgs;

async function sendChat({
  messages,
  model,
  abortSignal,
  stream,
  // lspUrl,
  // takeNote = false,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  tools,
}: SendChatArgs): Promise<Response> {
  // const toolsResponse = await getAvailableTools();

  // const tools = takeNote
  //   ? toolsResponse.filter(
  //       (tool) => tool.function.name === "remember_how_to_use_tools",
  //     )
  //   : toolsResponse.filter(
  //       (tool) => tool.function.name !== "remember_how_to_use_tools",
  //     );

  const body = JSON.stringify({
    messages,
    model: model,
    parameters: {
      max_new_tokens: 2048,
    },
    stream,
    tools,
    max_tokens: 2048,
    only_deterministic_messages,
    chat_id,
  });

  //   const apiKey = getApiKey();
  //   const headers = {
  //     "Content-Type": "application/json",
  //     ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  //   };
  //   const chatEndpoint = lspUrl
  //     ? `${lspUrl.replace(/\/*$/, "")}${CHAT_URL}`
  //     : CHAT_URL;

  return fetch("http://localhost:8001/v1/chat", {
    method: "POST",
    // headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    signal: abortSignal,
    credentials: "same-origin",
  });
}
