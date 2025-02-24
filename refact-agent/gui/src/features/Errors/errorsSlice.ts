import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type ErrorSliceState = { message: string | null; isAuthError?: boolean };

const initialState: ErrorSliceState = { message: null };
export const errorSlice = createSlice({
  name: "error",
  initialState,
  reducers: {
    setError: (state, action: PayloadAction<string>) => {
      if (state.message) return state;
      state.message = action.payload;
    },
    setIsAuthError: (state, action: PayloadAction<boolean>) => {
      state.isAuthError = action.payload;
    },
    clearError: (state, _action: PayloadAction) => {
      state.message = null;
    },
  },
  selectors: {
    getErrorMessage: (state) => state.message,
    getIsAuthError: (state) => state.isAuthError,
  },
});

export const { setError, setIsAuthError, clearError } = errorSlice.actions;
export const { getErrorMessage, getIsAuthError } = errorSlice.selectors;

// export const errorMiddleware = createListenerMiddleware();
// const startErrorListening = errorMiddleware.startListening.withTypes<
//   RootState,
//   AppDispatch
// >();

// startErrorListening({
//   // matcher: isAnyOf(chatError, isRejected),
//   // TODO: figure out why this breaks the tests when it's not a function :/
//   matcher: isAnyOf(isRejected),
//   effect: (action, listenerApi) => {
//     if (capsEndpoints.getCaps.matchRejected(action) && !action.meta.condition) {
//       const message = `fetching caps from lsp`;
//       listenerApi.dispatch(setError(message));
//     }

//     if (
//       promptsEndpoints.getPrompts.matchRejected(action) &&
//       !action.meta.condition
//     ) {
//       const message = `fetching system prompts.`;
//       listenerApi.dispatch(setError(action.error.message ?? message));
//     }

//     if (
//       chatAskQuestionThunk.rejected.match(action) &&
//       !action.meta.aborted &&
//       typeof action.payload === "string"
//     ) {
//       listenerApi.dispatch(setError(action.payload));
//     }
//   },
// });
