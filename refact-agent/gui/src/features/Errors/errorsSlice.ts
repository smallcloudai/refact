import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

const BALLANCE_LIMIT_MESSAGES = [
  '400 Bad Request: "💸 <b>Your balance is exhausted!</b>',
];
export type ErrorSliceState = {
  message: string | null;
  isAuthError?: boolean;
  type: "balance" | null;
};

const initialState: ErrorSliceState = { message: null, type: null };
export const errorSlice = createSlice({
  name: "error",
  initialState,
  reducers: {
    setError: (state, action: PayloadAction<string>) => {
      if (state.message) return state;
      state.message = action.payload;
      if (state.message.includes(BALLANCE_LIMIT_MESSAGES[0])) {
        state.type = "balance";
      }
    },
    setIsAuthError: (state, action: PayloadAction<boolean>) => {
      state.isAuthError = action.payload;
    },
    clearError: (state, _action: PayloadAction) => {
      state.message = null;
      state.type = null;
    },
  },
  selectors: {
    getErrorMessage: (state) => state.message,
    getIsAuthError: (state) => state.isAuthError,
    getErrorType: (state) => state.type,
  },
});

export const { setError, setIsAuthError, clearError } = errorSlice.actions;
export const { getErrorMessage, getIsAuthError, getErrorType } =
  errorSlice.selectors;
