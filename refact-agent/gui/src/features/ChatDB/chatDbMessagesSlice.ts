import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { CMessage } from "../../services/refact";

type InitialState = {
  threadId: string;
  messages: CMessage[];
  loading: boolean;
  error: null | string;
};

const initialState: InitialState = {
  threadId: "",
  messages: [],
  loading: false,
  error: null,
};

export const chatDbMessageSlice = createSlice({
  name: "chatDbMessages",
  initialState,
  reducers: {
    setThreadId: (state, action: PayloadAction<string>) => {
      state.threadId = action.payload;
    },
  },
});
