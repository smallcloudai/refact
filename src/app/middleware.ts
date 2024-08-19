import type { RootState, AppDispatch } from "./store";
import { createListenerMiddleware, isAnyOf } from "@reduxjs/toolkit";
import { newChatAction, restoreChat } from "../features/Chat";
import { statisticsApi } from "../services/refact/statistics";
import { capsApi } from "../services/refact/caps";
import { promptsApi } from "../services/refact/prompts";
import { toolsApi } from "../services/refact/tools";
import { commandsApi } from "../services/refact/commands";
import { diffApi } from "../services/refact/diffs";

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
  },
});
