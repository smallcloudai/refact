import { useEffect, useCallback, useRef, useMemo } from "react";
import { createReducer, createAction } from "@reduxjs/toolkit";
import { v4 as uuidv4 } from "uuid";
import {
  AssistantMessage,
  ChatContextFile,
  // ChatContextFileMessage,
  ChatMessage,
  ChatMessages,
  ChatRole,
  ContextMemory,
  DiffChunk,
  SystemPrompts,
  // LspChatMessage,
  ToolCall,
  ToolCommand,
  ToolResult,
  isAssistantDelta,
  isAssistantMessage,
  isChatContextFileDelta,
  isChatResponseChoice,
  isChatUserMessageResponse,
  isDiffMessage,
  isDiffResponse,
  isPlainTextResponse,
  isToolCallDelta,
  isToolMessage,
  isToolResponse,
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
import { type RootState } from "../../app/store";
import { parseOrElse } from "../../utils";
import { mergeToolCalls } from "../../hooks/useEventBusForChat/utils";

export type ChatThread = {
  id: string;
  messages: ChatMessages;
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
  system_prompt: SystemPrompts;
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
    system_prompt: {},
  };
};

const initialState = createInitialState();

type PayloadWIthId = { id: string };
export const newChatAction = createAction<PayloadWIthId>("chatThread/new");

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

export const setChatModel = createAction<PayloadWIthId & { model: string }>(
  "chatThread/setChatModel",
);
export const getSelectedChatModel = (state: RootState) =>
  state.chat.thread.model;

export const setSystemPrompt = createAction<SystemPrompts>(
  "chatThread/setSystemPrompt",
);

export const getSelectedSystemPrompt = (state: RootState) =>
  state.chat.system_prompt;

// ask question

export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(setChatModel, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.thread.model = action.payload.model;
  });

  builder.addCase(setSystemPrompt, (state, action) => {
    state.system_prompt = action.payload;
  });

  builder.addCase(newChatAction, (state, action) => {
    // TODO: save chat, or add to cache
    if (state.thread.id === action.payload.id) {
      const next = createInitialState();
      next.thread.model = state.thread.messages.length
        ? state.thread.model
        : "";
      state = next;
    }
  });

  builder.addCase(chatResponse, (state, action) => {
    // TODO: handle cache
    if (state.thread.id !== action.payload.id) return state;
    const hasUserMessage = isChatUserMessageResponse(action.payload);

    const current = hasUserMessage
      ? state.thread.messages.slice(0, state.previous_message_length)
      : state.thread.messages;

    // TODO: this might not be needed any more, because we can mutate the last message.
    const messages = formatChatResponse(current, action.payload);

    state.streaming = true;
    state.waiting_for_response = false;
    state.previous_message_length = messages.length;
    state.thread.messages = messages;
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

export function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  if (isChatUserMessageResponse(response)) {
    if (response.role === "context_file") {
      const content = parseOrElse<ChatContextFile[]>(response.content, []);
      // const msg: ChatContextFileMessage = { role: response.role, content };
      return [...messages, { role: response.role, content }];
    } else if (response.role === "context_memory") {
      const content = parseOrElse<ContextMemory[]>(response.content, []);
      return [...messages, { role: response.role, content }];
    }

    return [...messages, { role: response.role, content: response.content }];
  }

  if (isToolResponse(response)) {
    const { tool_call_id, content, finish_reason } = response;
    const toolResult: ToolResult = { tool_call_id, content, finish_reason };
    return [...messages, { role: response.role, content: toolResult }];
  }

  if (isDiffResponse(response)) {
    const content = parseOrElse<DiffChunk[]>(response.content, []);
    return [
      ...messages,
      { role: response.role, content, tool_call_id: response.tool_call_id },
    ];
  }

  if (isPlainTextResponse(response)) {
    return [...messages, response];
  }

  if (!isChatResponseChoice(response)) {
    // console.log("Not a good response");
    // console.log(response);
    return messages;
  }

  return response.choices.reduce<ChatMessages>((acc, cur) => {
    if (isChatContextFileDelta(cur.delta)) {
      const msg = { role: cur.delta.role, content: cur.delta.content };
      return acc.concat([msg]);
    }

    if (
      acc.length === 0 &&
      "content" in cur.delta &&
      typeof cur.delta.content === "string" &&
      cur.delta.role
    ) {
      if (cur.delta.role === "assistant") {
        const msg: AssistantMessage = {
          role: cur.delta.role,
          content: cur.delta.content,
          tool_calls: cur.delta.tool_calls,
        };
        return acc.concat([msg]);
      }
      // TODO: narrow this
      const message = {
        role: cur.delta.role,
        content: cur.delta.content,
      } as ChatMessage;
      return acc.concat([message]);
    }

    const lastMessage = acc[acc.length - 1];

    if (isToolCallDelta(cur.delta)) {
      if (!isAssistantMessage(lastMessage)) {
        return acc.concat([
          {
            role: "assistant",
            content: cur.delta.content ?? "",
            tool_calls: cur.delta.tool_calls,
          },
        ]);
      }

      const last = acc.slice(0, -1);
      const collectedCalls = lastMessage.tool_calls ?? [];
      const calls = mergeToolCalls(collectedCalls, cur.delta.tool_calls);
      const content = cur.delta.content;
      const message = content
        ? lastMessage.content + content
        : lastMessage.content;

      return last.concat([
        { role: "assistant", content: message, tool_calls: calls },
      ]);
    }

    if (
      isAssistantMessage(lastMessage) &&
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      const last = acc.slice(0, -1);
      const currentMessage = lastMessage.content ?? "";
      const toolCalls = lastMessage.tool_calls;
      return last.concat([
        {
          role: "assistant",
          content: currentMessage + cur.delta.content,
          tool_calls: toolCalls,
        },
      ]);
    } else if (
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      return acc.concat([{ role: "assistant", content: cur.delta.content }]);
    } else if (cur.delta.role === "assistant") {
      // empty message from JB
      return acc;
    }

    if (cur.delta.role === null || cur.finish_reason !== null) {
      return acc;
    }

    // console.log("Fall though");
    // console.log({ cur, lastMessage });

    return acc;
  }, messages);
}

export function formatMessagesForLsp(messages: ChatMessages): LspChatMessage[] {
  return messages.reduce<LspChatMessage[]>((acc, message) => {
    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message.role,
          content: message.content,
          tool_calls: message.tool_calls ?? undefined,
        },
      ]);
    }

    if (isToolMessage(message)) {
      return acc.concat([
        {
          role: "tool",
          content: message.content.content,
          tool_call_id: message.content.tool_call_id,
        },
      ]);
    }

    if (isDiffMessage(message)) {
      const diff = {
        role: message.role,
        content: JSON.stringify(message.content),
        tool_call_id: message.tool_call_id,
      };
      return acc.concat([diff]);
    }

    const content =
      typeof message.content === "string"
        ? message.content
        : JSON.stringify(message.content);
    return [...acc, { role: message.role, content }];
  }, []);
}

// TODO: make cancelable
const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    tools: ToolCommand[] | null;
  }
>("chatThread/sendChat", ({ messages, chatId, tools }, thunkAPI) => {
  const state = thunkAPI.getState();
  // const messagesWithPrompt =
  const messagesForLsp = formatMessagesForLsp(messages);
  sendChat({
    messages: messagesForLsp,
    model: state.chat.thread.model,
    tools,
    stream: true,
    abortSignal: thunkAPI.signal,
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

          // TODO: add better type checking
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
  const dispatch = useAppDispatch();
  const abortRef = useRef<null | ((reason?: string | undefined) => void)>(null);
  const capsRequest = useGetCapsQuery(undefined);
  const toolsRequest = useGetToolsQuery(!!capsRequest.data);

  const thread = useAppSelector((state) => state.chat.thread);
  const chatId = thread.id;
  const streaming = useAppSelector((state) => state.chat.streaming);
  const chatError = useAppSelector((state) => state.chat.error);
  const preventSend = useAppSelector((state) => state.chat.prevent_send);

  const currentMessages = useAppSelector((state) => state.chat.thread.messages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);

  const messagesWithSystemPrompt = useMemo(() => {
    const prompts = Object.entries(systemPrompt);
    if (prompts.length === 0) return currentMessages;
    const [key, prompt] = prompts[0];
    if (key === "default") return currentMessages;
    if (currentMessages.length === 0) {
      const message: ChatMessage = { role: "system", content: prompt.text };
      return [message];
    }
    return currentMessages;
  }, [currentMessages, systemPrompt]);

  const sendMessages = useCallback(
    (messages: ChatMessages) => {
      const tools = toolsRequest.data ?? null;
      dispatch(backUpMessages({ id: chatId, messages }));
      dispatch(chatAskedQuestion({ id: chatId }));

      const action = chatAskQuestionThunk({
        messages,
        tools,
        chatId,
      });

      const dispatchedAction = dispatch(action);
      abortRef.current = dispatchedAction.abort;
    },
    [chatId, dispatch, toolsRequest.data],
  );

  const submit = useCallback(
    (question: string) => {
      // const tools = toolsRequest.data ?? null;
      const message: ChatMessage = { role: "user", content: question };
      // This may cause duplicated messages
      const messages = messagesWithSystemPrompt.concat(message);
      sendMessages(messages);
    },
    [messagesWithSystemPrompt, sendMessages],
  );

  // TODO: retry
  const retry = useCallback(
    (messages: ChatMessages) => {
      sendMessages(messages);
    },
    [sendMessages],
  );

  // Automatically calls tool calls.
  useEffect(() => {
    if (
      !streaming &&
      currentMessages.length > 0 &&
      !chatError &&
      !preventSend
    ) {
      const lastMessage = currentMessages.slice(-1)[0];
      if (
        isAssistantMessage(lastMessage) &&
        lastMessage.tool_calls &&
        lastMessage.tool_calls.length > 0
      ) {
        sendMessages(currentMessages);
      }
    }
  }, [chatError, currentMessages, preventSend, sendMessages, streaming]);

  const abort = useCallback(() => {
    if (abortRef.current) {
      abortRef.current();
    }
  }, [abortRef]);

  useEffect(() => {
    if (!streaming && abortRef.current) {
      abortRef.current = null;
    }
  }, [streaming]);

  return {
    submit,
    abort,
    retry,
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

export type LspChatMessage = {
  role: ChatRole;
  content: string | null;
  tool_calls?: Omit<ToolCall, "index">[];
  tool_call_id?: string;
};

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

export const selectMessages = (state: RootState) => state.chat.thread.messages;
