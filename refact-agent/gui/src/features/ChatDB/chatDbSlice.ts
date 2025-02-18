import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { CThread } from "../../services/refact";
import { reset } from "../FIM";
import { setError } from "../Errors/errorsSlice";
import { getChatById } from "../History/historySlice";

export type ChatDbState = {
  loading: boolean;
  error: string | null;
  chats: Record<string, CThread>;
};

const initialState: ChatDbState = {
  loading: false,
  error: null,
  chats: {},
};

export const chatDbSlice = createSlice({
  name: "chatDb",
  initialState,
  reducers: {
    reset: () => initialState,
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.loading = action.payload;
    },
    setError: (state, action: PayloadAction<string>) => {
      state.error = action.payload;
    },
    startLoading: (state) => {
      state.loading = true;
      state.error = null;
      state.chats = {};
    },
    updateCThread: (state, action: PayloadAction<CThread>) => {
      state.chats[action.payload.cthread_id] = action.payload;
    },
    deleteCThread: (state, action: PayloadAction<string>) => {
      if (action.payload in state.chats) {
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
        delete state.chats[action.payload];
      }
    },
  },
  selectors: {
    getChats: (state) => state.chats,
    getLoading: (state) => state.loading,
    getError: (state) => state.error,
  },
});

export const chatDbActions = chatDbSlice.actions;
export const chatDbSelectors = chatDbSlice.selectors;
