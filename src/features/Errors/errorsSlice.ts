import {
  createSlice,
  PayloadAction,
  createListenerMiddleware,
  isAnyOf,
  isRejected,
} from "@reduxjs/toolkit";
import type { AppDispatch, RootState } from "../../app/store";
import { chatError } from "../Chat/chatThread";
import { capsEndpoints } from "../../services/refact/caps";
import { promptsEndpoints } from "../../services/refact/prompts";

const initialState: { message: string | null } = { message: null };
export const errorSlice = createSlice({
  name: "error",
  initialState,
  reducers: {
    setError: (state, action: PayloadAction<string>) => {
      if (state.message) return state;
      state.message = action.payload;
    },
    clearError: (state, _action: PayloadAction) => {
      state.message = null;
    },
  },
  selectors: {
    getErrorMessage: (state) => state.message,
  },
});

export const { setError, clearError } = errorSlice.actions;
export const { getErrorMessage } = errorSlice.selectors;

export const errorMiddleware = createListenerMiddleware();
const startErrorListening = errorMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startErrorListening({
  matcher: isAnyOf(chatError, isRejected),
  effect: (action, listenerApi) => {
    if (capsEndpoints.getCaps.matchRejected(action) && !action.meta.condition) {
      const message = `fetching caps from lsp.`;
      listenerApi.dispatch(setError(message));
    }

    if (
      promptsEndpoints.getPrompts.matchRejected(action) &&
      !action.meta.condition
    ) {
      const message = `fetching system prompts.`;
      listenerApi.dispatch(setError(message));
    }

    if (chatError.match(action)) {
      listenerApi.dispatch(setError(action.payload.message));
    }
  },
});
