import {
  createSlice,
  PayloadAction,
  createListenerMiddleware,
} from "@reduxjs/toolkit";
import {
  backUpMessages,
  chatAskedQuestion,
  chatGenerateTitleThunk,
  ChatThread,
  doneStreaming,
  removeChatFromCache,
  restoreChat,
} from "../Chat/Thread";
import { isAssistantMessage, isUserMessage } from "../../services/refact";
import { AppDispatch, RootState } from "../../app/store";

export type ChatHistoryItem = ChatThread & {
  createdAt: string;
  updatedAt: string;
  title: string;
};

export type HistoryMeta = Pick<
  ChatHistoryItem,
  "id" | "title" | "createdAt" | "model" | "updatedAt"
> & { userMessageCount: number };

export type HistoryState = Record<string, ChatHistoryItem>;

const initialState: HistoryState = {};

export const historySlice = createSlice({
  name: "history",
  initialState,
  reducers: {
    saveChat: (state, action: PayloadAction<ChatThread>) => {
      if (action.payload.messages.length === 0) return state;

      console.log(
        `[DEBUG]: action.payload.messages: `,
        action.payload.messages,
      );
      console.log(
        `[DEBUG]: action.payload.messages.length (UserMessage): `,
        action.payload.messages.filter(isUserMessage).length,
      );
      console.log(`[DEBUG]: action.payload.title: ${action.payload.title}`);

      const now = new Date().toISOString();
      const chat: ChatHistoryItem = {
        ...action.payload,
        title: action.payload.title ? action.payload.title : "New Chat",
        createdAt: action.payload.createdAt ?? now,

        updatedAt: now,
      };

      state[chat.id] = chat;
    },

    markChatAsUnread: (state, action: PayloadAction<string>) => {
      const chatId = action.payload;
      state[chatId].read = false;
    },

    markChatAsRead: (state, action: PayloadAction<string>) => {
      const chatId = action.payload;
      state[chatId].read = true;
    },

    deleteChatById: (state, action: PayloadAction<string>) => {
      return Object.entries(state).reduce<Record<string, ChatHistoryItem>>(
        (acc, [key, value]) => {
          if (key === action.payload) return acc;
          return { ...acc, [key]: value };
        },
        {},
      );
    },
  },
  selectors: {
    getChatById: (state, id: string): ChatHistoryItem | null => {
      if (!(id in state)) return null;
      return state[id];
    },

    getHistory: (state): ChatHistoryItem[] =>
      Object.values(state).sort((a, b) =>
        b.updatedAt.localeCompare(a.updatedAt),
      ),
  },
});

export const { saveChat, deleteChatById, markChatAsUnread, markChatAsRead } =
  historySlice.actions;
export const { getChatById, getHistory } = historySlice.selectors;

// We could use this or reduce-reducers packages
export const historyMiddleware = createListenerMiddleware();
const startHistoryListening = historyMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startHistoryListening({
  actionCreator: doneStreaming,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (state.chat.thread.id === action.payload.id) {
      listenerApi.dispatch(saveChat(state.chat.thread));
    } else if (action.payload.id in state.chat.cache) {
      listenerApi.dispatch(saveChat(state.chat.cache[action.payload.id]));
      listenerApi.dispatch(removeChatFromCache({ id: action.payload.id }));
    }

    if (state.chat.thread.messages.filter(isAssistantMessage).length === 1) {
      setTimeout(() => {
        listenerApi
          .dispatch(
            chatGenerateTitleThunk({
              messages: state.chat.thread.messages,
              chatId: action.payload.id,
            }),
          )
          .then((title) => {
            console.log(`[DEBUG]: title (then chain of thunk): `, title);
          })
          .catch((error) => {
            console.error(error);
          });
      }, 3000);
    }
  },
});

startHistoryListening({
  actionCreator: backUpMessages,
  effect: (action, listenerApi) => {
    const thread = listenerApi.getState().chat.thread;
    if (thread.id !== action.payload.id) return;
    const toSave = { ...thread, messages: action.payload.messages };
    listenerApi.dispatch(saveChat(toSave));
  },
});

startHistoryListening({
  actionCreator: chatAskedQuestion,
  effect: (action, listenerApi) => {
    listenerApi.dispatch(markChatAsUnread(action.payload.id));
  },
});

startHistoryListening({
  actionCreator: restoreChat,
  effect: (action, listenerApi) => {
    const chat = listenerApi.getState().chat;
    if (chat.thread.id == action.payload.id && chat.streaming) return;
    if (action.payload.id in chat.cache) return;
    listenerApi.dispatch(markChatAsRead(action.payload.id));
  },
});

// TODO: add a listener for creating a new chat ?
