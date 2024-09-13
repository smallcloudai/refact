import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import { type ChatThread, type PayloadWithId, type ToolUse } from "./types";
import {
  isAssistantMessage,
  isChatGetTitleResponse,
  type ChatMessages,
  type ChatResponse,
} from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
import type { SystemPrompts } from "../../../services/refact/prompts";
import { formatMessagesForLsp, consumeStream } from "./utils";
import { generateChatTitle, sendChat } from "../../../services/refact/chat";
import { ToolCommand } from "../../../services/refact/tools";

export const newChatAction = createAction("chatThread/new");

export const chatResponse = createAction<PayloadWithId & ChatResponse>(
  "chatThread/response",
);

export const chatAskedQuestion = createAction<PayloadWithId>(
  "chatThread/askQuestion",
);

export const backUpMessages = createAction<
  PayloadWithId & { messages: ChatThread["messages"] }
>("chatThread/backUpMessages");

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

// TODO: include history actions with this one, this could be done by making it a thunk, or use reduce-reducers.
export const doneStreaming = createAction<PayloadWithId>(
  "chatThread/doneStreaming",
);

export const setChatModel = createAction<string>("chatThread/setChatModel");
export const getSelectedChatModel = (state: RootState) =>
  state.chat.thread.model;

export const setSystemPrompt = createAction<SystemPrompts>(
  "chatThread/setSystemPrompt",
);

export const removeChatFromCache = createAction<PayloadWithId>(
  "chatThread/removeChatFromCache",
);

export const restoreChat = createAction<ChatThread>("chatThread/restoreChat");

export const clearChatError = createAction<PayloadWithId>(
  "chatThread/clearError",
);

export const enableSend = createAction<PayloadWithId>("chatThread/enableSend");
export const setPreventSend = createAction<PayloadWithId>(
  "chatThread/preventSend",
);

export const setToolUse = createAction<ToolUse>("chatThread/setToolUse");
export const getChatTitle = createAction<PayloadWithId>(
  "chatThread/getChatTitle",
);
// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

export const chatGenerateTitleThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
  }
>("chatThread/generateTitle", ({ messages, chatId }, thunkAPI) => {
  const state = thunkAPI.getState();

  const messagesForLsp = formatMessagesForLsp([
    ...messages,
    {
      role: "user",
      content:
        "Generate a short 2-3 word title for current chat based on its context",
    },
  ]);

  return generateChatTitle({
    messages: messagesForLsp,
    model: state.chat.thread.model,
    stream: false,
    chatId,
    apiKey: state.config.apiKey,
    port: state.config.lspPort,
  })
    .then((response) => {
      if (!response.ok) {
        return Promise.reject(new Error(response.statusText));
      }
      return response.json();
    })
    .then((data) => {
      if (!isChatGetTitleResponse(data)) {
        return;
      }

      const title = data.choices[0].message.content;
      const cleanedTitle = title.replace(/"/g, "");

      return { title: cleanedTitle, chatId: state.chat.thread.id };
    })
    .catch((err: Error) => {
      // console.log("Catch called");
      thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
      return thunkAPI.rejectWithValue(err.message);
    });
});

export const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    tools: ToolCommand[] | null;
  }
>("chatThread/sendChat", ({ messages, chatId, tools }, thunkAPI) => {
  const state = thunkAPI.getState();

  const messagesForLsp = formatMessagesForLsp(messages);
  return sendChat({
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

      const reader = response.body?.getReader();
      if (!reader) return;
      const onAbort = () => thunkAPI.dispatch(setPreventSend({ id: chatId }));
      const onChunk = (json: Record<string, unknown>) => {
        const action = chatResponse({ ...(json as ChatResponse), id: chatId });
        return thunkAPI.dispatch(action);
      };
      return consumeStream(reader, thunkAPI.signal, onAbort, onChunk);
    })
    .catch((err: Error) => {
      // console.log("Catch called");
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
      thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
      return thunkAPI.rejectWithValue(err.message);
    })
    .finally(() => {
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
      if (state.chat.thread.messages.filter(isAssistantMessage).length === 1) {
        thunkAPI
          .dispatch(
            chatGenerateTitleThunk({
              messages: state.chat.thread.messages,
              chatId,
            }),
          )
          .then(() => {
            return;
          })
          .catch(() => {
            return;
          });
      }
    });
});
