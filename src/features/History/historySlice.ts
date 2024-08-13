import {
  createSlice,
  PayloadAction,
  createListenerMiddleware,
} from "@reduxjs/toolkit";
import { ChatThread, doneStreaming, removeChatFromCache } from "../Chat";
import { isUserMessage, UserMessage } from "../../services/refact";
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

      const userMessage: UserMessage | undefined =
        action.payload.messages.find(isUserMessage);
      if (!userMessage) return state;

      const now = new Date().toISOString();
      const chat: ChatHistoryItem = {
        ...action.payload,
        title: action.payload.title
          ? action.payload.title
          : userMessage.content.replace(/^\W*/, "") || "New Chat",
        createdAt: action.payload.createdAt ?? now,

        updatedAt: now,
      };

      // TODO: handle storage overflow (in redux persist)
      state[chat.id] = chat;
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
        a.updatedAt.localeCompare(b.updatedAt),
      ),
  },
});

export const { saveChat, deleteChatById } = historySlice.actions;
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
  },
});

// TODO: add a listener for creating a new chat ?
