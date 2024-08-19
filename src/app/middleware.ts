import type { RootState, AppDispatch } from "./store";
import {
  createListenerMiddleware,
  isAnyOf,
  isRejected,
} from "@reduxjs/toolkit";
import {
  chatAskQuestionThunk,
  newChatAction,
  restoreChat,
} from "../features/Chat/chatThread";
import { statisticsApi } from "../services/refact/statistics";
import { capsApi } from "../services/refact/caps";
import { promptsApi } from "../services/refact/prompts";
import { toolsApi } from "../services/refact/tools";
import { commandsApi } from "../services/refact/commands";
import { diffApi } from "../services/refact/diffs";
import { clearError, setError } from "../features/Errors/errorsSlice";

export const listenerMiddleware = createListenerMiddleware();
const startErrorListening = listenerMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startErrorListening({
  // TODO: figure out why this breaks the tests when it's not a function :/
  matcher: isAnyOf(
    (d: unknown): d is ReturnType<typeof newChatAction> =>
      newChatAction.match(d),
    (d: unknown): d is ReturnType<typeof restoreChat> => restoreChat.match(d),
  ),
  effect: (_action, listenerApi) => {
    [
      statisticsApi.util.resetApiState(),
      capsApi.util.resetApiState(),
      promptsApi.util.resetApiState(),
      toolsApi.util.resetApiState(),
      commandsApi.util.resetApiState(),
      diffApi.util.resetApiState(),
    ].forEach((api) => listenerApi.dispatch(api));

    listenerApi.dispatch(clearError());
  },
});

startErrorListening({
  // matcher: isAnyOf(chatError, isRejected),
  // TODO: figure out why this breaks the tests when it's not a function :/
  matcher: isAnyOf(isRejected),
  effect: (action, listenerApi) => {
    if (
      capsApi.endpoints.getCaps.matchRejected(action) &&
      !action.meta.condition
    ) {
      const message = `fetching caps from lsp`;
      listenerApi.dispatch(setError(message));
    }

    if (
      promptsApi.endpoints.getPrompts.matchRejected(action) &&
      !action.meta.condition
    ) {
      const message = `fetching system prompts.`;
      listenerApi.dispatch(setError(action.error.message ?? message));
    }

    if (
      chatAskQuestionThunk.rejected.match(action) &&
      !action.meta.aborted &&
      typeof action.payload === "string"
    ) {
      listenerApi.dispatch(setError(action.payload));
    }
  },
});
