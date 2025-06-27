import { createListenerMiddleware } from "@reduxjs/toolkit/react";
import { type AppDispatch, type RootState } from "../../app/store";
import { threadMessagesSlice } from "../ThreadMessages";
import { setExpert, setModel } from "./expertsSlice";

export const expertsAndModelsMiddleWare = createListenerMiddleware();
const startListening = expertsAndModelsMiddleWare.startListening.withTypes<
  RootState,
  AppDispatch
>();

startListening({
  actionCreator: threadMessagesSlice.actions.receiveThread,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (
      !state.threadMessages.ft_id ||
      !action.payload.news_payload_id.startsWith(state.threadMessages.ft_id)
    ) {
      return;
    }

    if (
      action.payload.news_payload_thread.ft_fexp_id &&
      action.payload.news_payload_thread.ft_fexp_id !==
        state.experts.selectedExpert
    ) {
      listenerApi.dispatch(
        setExpert(action.payload.news_payload_thread.ft_fexp_id),
      );
    }
  },
});

startListening({
  actionCreator: threadMessagesSlice.actions.receiveThreadMessages,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (
      !state.threadMessages.ft_id ||
      !action.payload.news_payload_id.startsWith(state.threadMessages.ft_id)
    ) {
      return;
    }

    const maybeModel = getModel(action.payload);
    if (maybeModel && maybeModel !== state.experts.selectedModel) {
      listenerApi.dispatch(setModel(maybeModel));
    }
  },
});

function getModel(preferences: unknown): string | null {
  if (!preferences) return null;
  if (typeof preferences !== "object") return null;
  if (!("model" in preferences)) {
    return null;
  }
  if (typeof preferences.model !== "string") {
    return null;
  }
  return preferences.model;
}
