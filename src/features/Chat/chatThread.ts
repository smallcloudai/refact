import { useEffect, useCallback, useRef, useMemo } from "react";
import {
  createReducer,
  createAction,
  createAsyncThunk,
} from "@reduxjs/toolkit";
import { v4 as uuidv4 } from "uuid";
import {
  ChatMessage,
  ChatMessages,
  SystemPrompts,
  ToolCommand,
  isAssistantMessage,
  isChatUserMessageResponse,
} from "../../services/refact";
// TODO: update this type
import type { ChatResponse } from "../../services/refact";
import {
  useAppDispatch,
  // createAppAsyncThunk,
  useAppSelector,
  useGetToolsQuery,
} from "../../app/hooks";
import { type AppDispatch, type RootState } from "../../app/store";
import { parseOrElse } from "../../utils";
import { formatChatResponse, formatMessagesForLsp } from "./utils";
import { sendChat } from "../../services/refact";

export type ChatThread = {
  id: string;
  messages: ChatMessages;
  model: string;
  title?: string;
  // attach_file?: boolean;
  createdAt?: string;
  updatedAt?: string;
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
  use_tools: boolean;
  send_immediately: boolean;
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
    use_tools: true,
    send_immediately: false,
  };
};

const initialState = createInitialState();

type PayloadWIthId = { id: string };
// TODO: add history actions to this
export const newChatAction = createAction<PayloadWIthId>("chatThread/new");

const chatResponse = createAction<PayloadWIthId & ChatResponse>(
  "chatThread/response",
);

const chatAskedQuestion = createAction<PayloadWIthId>("chatThread/askQuestion");

// TODO: does this need history actions?
const backUpMessages = createAction<
  PayloadWIthId & { messages: ChatThread["messages"] }
>("chatThread/backUpMessages");

// TODO: add history actions to this
export const chatError = createAction<PayloadWIthId & { message: string }>(
  "chatThread/error",
);

// TODO: include history actions with this one, this could be done by making it a thunk, or use reduce-reducers.
export const doneStreaming = createAction<PayloadWIthId>(
  "chatThread/doneStreaming",
);

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

export const removeChatFromCache = createAction<PayloadWIthId>(
  "chatThread/removeChatFromCache",
);

export const restoreChat = createAction<PayloadWIthId & { thread: ChatThread }>(
  "chatThread/restoreChat",
);

export const clearChatError = createAction<PayloadWIthId>(
  "chatThread/clearError",
);

export const enableSend = createAction<PayloadWIthId>("chatThread/enableSend");

export const setUseTools = createAction<boolean>("chatThread/setUseTools");

export const chatReducer = createReducer(initialState, (builder) => {
  builder.addCase(setUseTools, (state, action) => {
    state.use_tools = action.payload;
  });

  builder.addCase(enableSend, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.prevent_send = false;
  });

  builder.addCase(clearChatError, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.error = null;
  });

  builder.addCase(setChatModel, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    state.thread.model = action.payload.model;
  });

  builder.addCase(setSystemPrompt, (state, action) => {
    state.system_prompt = action.payload;
  });

  builder.addCase(newChatAction, (state, action) => {
    if (state.thread.id !== action.payload.id) return state;
    if (state.streaming) {
      state.cache[state.thread.id] = state.thread;
    }
    const next = createInitialState();
    next.thread.model = state.thread.messages.length ? state.thread.model : "";
    return next;
  });

  builder.addCase(chatResponse, (state, action) => {
    if (
      action.payload.id !== state.thread.id &&
      !(action.payload.id in state.cache)
    ) {
      return state;
    }

    if (action.payload.id in state.cache) {
      const thread = state.cache[action.payload.id];
      // TODO: this might not be needed any more, because we can mutate the last message.
      const messages = formatChatResponse(thread.messages, action.payload);
      state.thread.messages = messages;
      return state;
    }

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
    state.send_immediately = false;
    state.waiting_for_response = true;
    state.streaming = true;
  });

  builder.addCase(removeChatFromCache, (state, action) => {
    if (!(action.payload.id in state.cache)) return state;

    const cache = Object.entries(state.cache).reduce<
      Record<string, ChatThread>
    >((acc, cur) => {
      if (cur[0] === action.payload.id) return acc;
      return { ...acc, [cur[0]]: cur[1] };
    }, {});
    state.cache = cache;
  });

  builder.addCase(restoreChat, (state, action) => {
    if (action.payload.id !== state.thread.id) return state;
    if (state.streaming) {
      state.cache[state.thread.id] = state.thread;
      state.streaming = false;
    }
    state.error = null;
    state.waiting_for_response = false;
    state.previous_message_length = action.payload.thread.messages.length;
    state.thread = action.payload.thread;
  });
});

// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

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
    apiKey: state.config.apiKey,
    port: state.config.lspPort,
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
        if (thunkAPI.signal.aborted) {
          console.log("Aborted");
          return Promise.resolve();
        }

        const streamAsString = decoder.decode(value);

        const maybeError = checkForDetailMessage(streamAsString);
        if (maybeError) {
          const error = new Error(maybeError.detail);
          throw error;
        }

        const deltas = streamAsString
          .split("\n\n")
          .filter((str) => str.length > 0);

        if (deltas.length === 0) return Promise.resolve();

        // could be improved
        for (const delta of deltas) {
          // can have error here.
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

          const maybeErrorData = checkForDetailMessage(maybeJsonString);
          if (maybeErrorData) {
            const errorMessage: string =
              typeof maybeErrorData.detail === "string"
                ? maybeErrorData.detail
                : JSON.stringify(maybeErrorData.detail);
            const error = new Error(errorMessage);
            // eslint-disable-next-line no-console
            console.error(error);
            throw error;
          }
          const json = parseOrElse<Record<string, unknown>>(
            maybeJsonString,
            {},
          );

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

type DetailMessage = { detail: string };

function checkForDetailMessage(str: string): DetailMessage | false {
  const json = parseOrElse(str, {});
  if ("detail" in json) return json as DetailMessage;
  return false;
}

export const selectThread = (state: RootState) => state.chat.thread;
export const selectChatId = (state: RootState) => state.chat.thread.id;
export const selectModel = (state: RootState) => state.chat.thread.model;
export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectUseTools = (state: RootState) => state.chat.use_tools;
export const selectIsWaiting = (state: RootState) =>
  state.chat.waiting_for_response;
export const selectIsStreaming = (state: RootState) => state.chat.streaming;
export const selectPreventSend = (state: RootState) => state.chat.prevent_send;
export const selectChatError = (state: RootState) => state.chat.error;
export const selectSendImmediately = (state: RootState) =>
  state.chat.send_immediately;

export const useSendChatRequest = () => {
  const dispatch = useAppDispatch();
  const abortRef = useRef<null | ((reason?: string | undefined) => void)>(null);

  const toolsRequest = useGetToolsQuery();

  const chatId = useAppSelector(selectChatId);
  const streaming = useAppSelector(selectIsStreaming);
  const chatError = useAppSelector(selectChatError);
  const preventSend = useAppSelector(selectPreventSend);

  const currentMessages = useAppSelector(selectMessages);
  const systemPrompt = useAppSelector(getSelectedSystemPrompt);
  const sendImmediately = useAppSelector(selectSendImmediately);

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

  useEffect(() => {
    if (sendImmediately) {
      sendMessages(messagesWithSystemPrompt);
    }
  }, [sendImmediately, sendMessages, messagesWithSystemPrompt]);

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

  const abort = () => {
    if (abortRef.current) {
      abortRef.current();
    }
  };

  return {
    submit,
    abort,
    retry,
  };
};
