import type { RootState, AppDispatch } from "./store";
import {
  createListenerMiddleware,
  isAnyOf,
  isRejected,
} from "@reduxjs/toolkit";
import {
  doneStreaming,
  newChatAction,
  chatAskQuestionThunk,
  restoreChat,
  newIntegrationChat,
} from "../features/Chat/Thread";
import { statisticsApi } from "../services/refact/statistics";
import { integrationsApi } from "../services/refact/integrations";
import { dockerApi } from "../services/refact/docker";
import { capsApi, isCapsErrorResponse } from "../services/refact/caps";
import { promptsApi } from "../services/refact/prompts";
import { toolsApi } from "../services/refact/tools";
import { commandsApi, isDetailMessage } from "../services/refact/commands";
import { pathApi } from "../services/refact/path";
import { diffApi } from "../services/refact/diffs";
import { pingApi } from "../services/refact/ping";
import { clearError, setError } from "../features/Errors/errorsSlice";
import { updateConfig } from "../features/Config/configSlice";
import { resetAttachedImagesSlice } from "../features/AttachedImages";
import { nextTip } from "../features/TipOfTheDay";

export const listenerMiddleware = createListenerMiddleware();
const startListening = listenerMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startListening({
  // TODO: figure out why this breaks the tests when it's not a function :/
  matcher: isAnyOf(
    (d: unknown): d is ReturnType<typeof newChatAction> =>
      newChatAction.match(d),
    (d: unknown): d is ReturnType<typeof restoreChat> => restoreChat.match(d),
  ),
  effect: (_action, listenerApi) => {
    [
      pingApi.util.resetApiState(),
      statisticsApi.util.resetApiState(),
      capsApi.util.resetApiState(),
      promptsApi.util.resetApiState(),
      toolsApi.util.resetApiState(),
      commandsApi.util.resetApiState(),
      diffApi.util.resetApiState(),
      resetAttachedImagesSlice(),
    ].forEach((api) => listenerApi.dispatch(api));

    listenerApi.dispatch(clearError());
  },
});

startListening({
  // TODO: figure out why this breaks the tests when it's not a function :/
  matcher: isAnyOf(isRejected),
  effect: (action, listenerApi) => {
    if (
      capsApi.endpoints.getCaps.matchRejected(action) &&
      !action.meta.condition
    ) {
      const message = isCapsErrorResponse(action.payload?.data)
        ? action.payload.data.detail
        : `fetching caps from lsp`;
      listenerApi.dispatch(setError(message));
    }
    if (
      toolsApi.endpoints.getTools.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting error message from LSP
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : "fetching tools from lsp.";
      listenerApi.dispatch(setError(errorMessage));
    }
    if (
      toolsApi.endpoints.checkForConfirmation.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : "confirmation check from lsp";
      listenerApi.dispatch(setError(errorMessage));
    }
    if (
      promptsApi.endpoints.getPrompts.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail.split("\n").slice(0, 2).join("\n")
        : `fetching system prompts.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      integrationsApi.endpoints.getAllIntegrations.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching integrations.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      integrationsApi.endpoints.getIntegrationByPath.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching integrations.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      dockerApi.endpoints.getAllDockerContainers.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching docker containers.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      dockerApi.endpoints.getDockerContainersByImage.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching docker containers.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      dockerApi.endpoints.getDockerContainersByLabel.matchRejected(action) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching docker containers.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      dockerApi.endpoints.executeActionForDockerContainer.matchRejected(
        action,
      ) &&
      !action.meta.condition
    ) {
      // getting first 2 lines of error message to show to user
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `fetching docker containers.`;
      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      pathApi.endpoints.getFullPath.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : "getting fullpath of file";

      listenerApi.dispatch(setError(errorMessage));
    }

    if (
      chatAskQuestionThunk.rejected.match(action) &&
      !action.meta.aborted &&
      typeof action.payload === "string"
    ) {
      listenerApi.dispatch(setError(action.payload));
    }

    if (diffApi.endpoints.applyAllPatchesInMessages.matchRejected(action)) {
      const errorMessage = isDetailMessage(action.payload?.data)
        ? action.payload.data.detail
        : `Failed to apply diffs: ${action.payload?.status}`;
      listenerApi.dispatch(setError(errorMessage));
    }
  },
});

startListening({
  actionCreator: updateConfig,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (
      action.payload.apiKey !== state.config.apiKey ||
      action.payload.addressURL !== state.config.addressURL ||
      action.payload.lspPort !== state.config.lspPort
    ) {
      pingApi.util.resetApiState();
    }
  },
});

startListening({
  actionCreator: doneStreaming,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (action.payload.id === state.chat.thread.id) {
      listenerApi.dispatch(resetAttachedImagesSlice());
    }
  },
});

startListening({
  matcher: isAnyOf(restoreChat, newChatAction, updateConfig),
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const isUpdate = updateConfig.match(action);

    const host =
      isUpdate && action.payload.host ? action.payload.host : state.config.host;

    const completeManual = isUpdate
      ? action.payload.keyBindings?.completeManual
      : state.config.keyBindings?.completeManual;

    listenerApi.dispatch(
      nextTip({
        host,
        completeManual,
      }),
    );
  },
});

startListening({
  actionCreator: newIntegrationChat,
  effect: async (_action, listenerApi) => {
    const state = listenerApi.getState();

    // TBD: should the endpoint need tools?
    const toolsRequest = listenerApi.dispatch(
      toolsApi.endpoints.getTools.initiate(undefined),
    );
    toolsRequest.unsubscribe();
    const toolResult = await toolsRequest.unwrap();
    // TODO: set mode to configure ? or infer it later
    // TODO: create a dedicated thunk for this.
    await listenerApi.dispatch(
      chatAskQuestionThunk({
        messages: state.chat.thread.messages,
        chatId: state.chat.thread.id,
        tools: toolResult,
      }),
    );
  },
});
