import { createListenerMiddleware } from "@reduxjs/toolkit/react";
import { type AppDispatch, type RootState } from "../../app/store";
import { threadMessagesSlice } from "../ThreadMessages";
import { setExpert, setModel } from "./expertsSlice";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";

export const expertsAndModelsMiddleWare = createListenerMiddleware();
const startListening = expertsAndModelsMiddleWare.startListening.withTypes<
  RootState,
  AppDispatch
>();

startListening({
  actionCreator: threadMessagesSlice.actions.receiveThreadMessages,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (
      !state.threadMessages.ft_id ||
      !action.payload.comprehensive_thread_subs.news_payload_id.startsWith(
        state.threadMessages.ft_id,
      )
    ) {
      return;
    }

    if (
      action.payload.comprehensive_thread_subs.news_payload_thread
        ?.ft_fexp_id &&
      action.payload.comprehensive_thread_subs.news_payload_thread
        .ft_fexp_id !== state.experts.selectedExpert
    ) {
      listenerApi.dispatch(
        setExpert(
          action.payload.comprehensive_thread_subs.news_payload_thread
            .ft_fexp_id,
        ),
      );
    }

    action.payload.comprehensive_thread_subs;

    const maybeModel = getModel(action.payload);
    if (maybeModel && maybeModel !== state.experts.selectedModel) {
      listenerApi.dispatch(setModel(maybeModel));
    }
  },
});

function getModel(message: MessagesSubscriptionSubscription): string | null {
  const thread = message.comprehensive_thread_subs;
  if (!thread.news_payload_thread_message) return null;
  if (
    typeof thread.news_payload_thread_message.ftm_user_preferences !== "object"
  ) {
    return null;
  }
  if (!thread.news_payload_thread_message.ftm_user_preferences) return null;
  const preferences = thread.news_payload_thread_message
    .ftm_user_preferences as Record<string, unknown>;

  if (!("model" in preferences)) {
    return null;
  }
  if (typeof preferences.model !== "string") {
    return null;
  }
  return preferences.model;
}
