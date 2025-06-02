import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export const threadMessagesSlice = createSlice({
  name: "threadMessages",
  initialState: {
    messages: [],
  },
  reducers: {
    addMessage: (state, action: PayloadAction<string>) => {
      state.messages.push(action.payload);
    },
  },
});
