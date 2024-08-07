import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { ChatThread } from "../Chat2/chatThread";
import { isUserMessage, UserMessage } from "../../events";

export type ChatHistoryItem = ChatThread & {
  createdAt: string;
  updatedAt: string;
  title: string;
};

export type HistoryMeta = Pick<
  ChatHistoryItem,
  "id" | "title" | "createdAt" | "model" | "updatedAt"
>;

const initialState: Record<string, ChatHistoryItem> = {};

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
        title:
          action.payload.title ??
          (userMessage.content.replace(/^\W*/, "") || "New Chat"),
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

    getHistoryMeta: (state): HistoryMeta[] =>
      Object.values(state)
        .sort((a, b) => a.updatedAt.localeCompare(b.updatedAt))
        .map((item) => {
          const { id, title, createdAt, model, updatedAt } = item;
          return { id, title, createdAt, model, updatedAt };
        }),
  },
});

export const { saveChat, deleteChatById } = historySlice.actions;
export const { getChatById, getHistoryMeta } = historySlice.selectors;
