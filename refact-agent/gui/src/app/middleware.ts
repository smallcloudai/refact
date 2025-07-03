import type { RootState, AppDispatch } from "./store";
import {
  createListenerMiddleware,
  isAnyOf,
  isRejected,
} from "@reduxjs/toolkit";

import { statisticsApi } from "../services/refact/statistics";
import { integrationsApi } from "../services/refact/integrations";
import { dockerApi } from "../services/refact/docker";
import { toolsApi } from "../services/refact/tools";
import { commandsApi, isDetailMessage } from "../services/refact/commands";
import { pathApi } from "../services/refact/path";
import { pingApi } from "../services/refact/ping";
import {
  clearError,
  setError,
  setIsAuthError,
} from "../features/Errors/errorsSlice";
import { setThemeMode, updateConfig } from "../features/Config/configSlice";
import { resetAttachedImagesSlice } from "../features/AttachedImages";
import { nextTip } from "../features/TipOfTheDay";
import { telemetryApi } from "../services/refact/telemetry";
import { CONFIG_PATH_URL, FULL_PATH_URL } from "../services/refact/consts";
import {
  ideToolCallResponse,
  ideForceReloadProjectTreeFiles,
} from "../hooks/useEventBusForIDE";

import { isToolMessage, modelsApi, providersApi } from "../services/refact";
import {
  selectLastMessageForAlt,
  selectMessageByToolCallId,
  selectToolConfirmationRequests,
  threadMessagesSlice,
} from "../features/ThreadMessages";
import {
  createMessage,
  createThreadWithMessage,
  createThreadWitMultipleMessages,
  rejectToolUsageAction,
  toolConfirmationThunk,
} from "../services/graphql/graphqlThunks";
import { push } from "../features/Pages/pagesSlice";

const AUTH_ERROR_MESSAGE =
  "There is an issue with your API key. Check out your API Key or re-login";

export const listenerMiddleware = createListenerMiddleware();
const startListening = listenerMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startListening({
  // TODO: figure out why this breaks the tests when it's not a function :/
  // matcher: isAnyOf(
  //   (d: unknown): d is ReturnType<typeof newChatAction> =>
  //     newChatAction.match(d),
  //   // (d: unknown): d is ReturnType<typeof restoreChat> => restoreChat.match(d),
  // ),
  actionCreator: threadMessagesSlice.actions.resetThread,
  effect: (_action, listenerApi) => {
    [
      // pingApi.util.resetApiState(),
      statisticsApi.util.resetApiState(),
      // capsApi.util.resetApiState(),
      // promptsApi.util.resetApiState(),
      toolsApi.util.resetApiState(),
      commandsApi.util.resetApiState(),
      resetAttachedImagesSlice(),
      clearError(),
    ].forEach((api) => listenerApi.dispatch(api));

    listenerApi.dispatch(clearError());
  },
});

startListening({
  // TODO: figure out why this breaks the tests when it's not a function :/
  matcher: isAnyOf(isRejected),
  effect: (action, listenerApi) => {
    if (
      toolsApi.endpoints.getToolGroups.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching tool groups from lsp`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      integrationsApi.endpoints.getAllIntegrations.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching integrations.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      integrationsApi.endpoints.deleteIntegration.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `deleting integrations.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      integrationsApi.endpoints.getIntegrationByPath.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching integrations.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      dockerApi.endpoints.getAllDockerContainers.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching docker containers.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      dockerApi.endpoints.getDockerContainersByImage.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching docker containers.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      dockerApi.endpoints.getDockerContainersByLabel.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching docker containers.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      dockerApi.endpoints.executeActionForDockerContainer.matchRejected(
        action,
      ) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching docker containers.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    if (
      pathApi.endpoints.getFullPath.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `getting full path of file.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }

    // TODO: thread or message error?
    if (
      (createThreadWitMultipleMessages.rejected.match(action) ||
        createThreadWithMessage.rejected.match(action) ||
        createMessage.rejected.match(action)) &&
      !action.meta.aborted &&
      typeof action.payload === "string"
    ) {
      listenerApi.dispatch(setError(action.payload));
    }

    if (
      (providersApi.endpoints.updateProvider.matchRejected(action) ||
        providersApi.endpoints.getProvider.matchRejected(action) ||
        providersApi.endpoints.getProviderTemplates.matchRejected(action) ||
        providersApi.endpoints.getConfiguredProviders.matchRejected(action)) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `provider update error.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }
    if (
      modelsApi.endpoints.getModels.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `provider update error.`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }
  },
});

startListening({
  actionCreator: updateConfig,
  effect: (_action, listenerApi) => {
    listenerApi.dispatch(pingApi.util.resetApiState());
  },
});

startListening({
  matcher: isAnyOf(
    createMessage.fulfilled,
    createThreadWithMessage.fulfilled,
    createThreadWitMultipleMessages.fulfilled,
  ),
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (
      createMessage.fulfilled.match(action) &&
      action.meta.arg.input.ftm_belongs_to_ft_id ===
        state.threadMessages.thread?.ft_id
    ) {
      listenerApi.dispatch(resetAttachedImagesSlice());
    } else if (
      createThreadWithMessage.fulfilled.match(action) &&
      action.payload.thread_create.ft_id === state.threadMessages.ft_id
    ) {
      listenerApi.dispatch(resetAttachedImagesSlice());
    } else if (
      createThreadWitMultipleMessages.fulfilled.match(action) &&
      action.payload.thread_create.ft_id !== state.threadMessages.ft_id
    ) {
      listenerApi.dispatch(resetAttachedImagesSlice());
    }
  },
});

startListening({
  matcher: isAnyOf(
    // restoreChat,
    // newChatAction,
    updateConfig,
    threadMessagesSlice.actions.resetThread,
  ),
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

// An integration chat was started.
startListening({
  actionCreator: createThreadWitMultipleMessages.fulfilled,
  effect: (action, listenerApi) => {
    if (action.meta.arg.integration) {
      listenerApi.dispatch(integrationsApi.util.resetApiState());
      listenerApi.dispatch(clearError());
      listenerApi.dispatch(
        push({ name: "chat", ft_id: action.payload.thread_create.ft_id }),
      );
    }
  },
});

// Telemetry
startListening({
  matcher: isAnyOf(
    pathApi.endpoints.getFullPath.matchFulfilled,
    pathApi.endpoints.getFullPath.matchRejected,
    pathApi.endpoints.customizationPath.matchFulfilled,
    pathApi.endpoints.customizationPath.matchRejected,
    pathApi.endpoints.privacyPath.matchFulfilled,
    pathApi.endpoints.privacyPath.matchRejected,
    pathApi.endpoints.integrationsPath.matchFulfilled,
    pathApi.endpoints.integrationsPath.matchRejected,
  ),
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();

    if (pathApi.endpoints.getFullPath.matchFulfilled(action)) {
      const thunk = telemetryApi.endpoints.sendTelemetryNetEvent.initiate({
        url: FULL_PATH_URL,
        scope: "getFullPath",
        success: true,
        error_message: "",
      });
      void listenerApi.dispatch(thunk);
    }

    if (
      pathApi.endpoints.getFullPath.matchRejected(action) &&
      !action.meta.condition
    ) {
      const thunk = telemetryApi.endpoints.sendTelemetryNetEvent.initiate({
        url: FULL_PATH_URL,
        scope: "getFullPath",
        success: false,
        error_message: action.error.message ?? JSON.stringify(action.error),
      });
      void listenerApi.dispatch(thunk);
    }

    if (
      pathApi.endpoints.customizationPath.matchFulfilled(action) ||
      pathApi.endpoints.privacyPath.matchFulfilled(action) ||
      pathApi.endpoints.integrationsPath.matchFulfilled(action)
    ) {
      const thunk = telemetryApi.endpoints.sendTelemetryNetEvent.initiate({
        url: CONFIG_PATH_URL,
        scope: action.meta.arg.endpointName,
        success: true,
        error_message: "",
      });
      void listenerApi.dispatch(thunk);
    }

    if (
      (pathApi.endpoints.customizationPath.matchRejected(action) ||
        pathApi.endpoints.privacyPath.matchRejected(action) ||
        pathApi.endpoints.integrationsPath.matchRejected(action)) &&
      !action.meta.condition
    ) {
      const thunk = telemetryApi.endpoints.sendTelemetryNetEvent.initiate({
        url: CONFIG_PATH_URL,
        scope: action.meta.arg.endpointName,
        success: false,
        error_message: action.error.message ?? JSON.stringify(action.error),
      });
      void listenerApi.dispatch(thunk);
    }
  },
});

// TODO: this should let flexus know that the user accepted the tool
// Tool Call results from ide.
startListening({
  actionCreator: ideToolCallResponse,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();

    // TODO: reject, will require making a new message so the chat must be loaded
    if (state.threadMessages.thread?.ft_id !== action.payload.chatId) return;

    // Check if already confirmed
    const pendingRequests = selectToolConfirmationRequests(state);
    const maybePendingToolCall = pendingRequests.find(
      (req) => req.tool_call_id === action.payload.toolCallId,
    );
    if (!maybePendingToolCall) return;

    if (action.payload.accepted) {
      const thunk = toolConfirmationThunk({
        ft_id: action.payload.chatId,
        confirmation_response: JSON.stringify([action.payload.toolCallId]),
      });
      void listenerApi.dispatch(thunk);
      return;
    }

    // rejection creates a new message at the end of the thread
    // find the parent, then find the end point
    const message = selectMessageByToolCallId(state, action.payload.toolCallId);
    if (!message) return;
    const lastMessage = selectLastMessageForAlt(state, message.ftm_alt);
    if (!lastMessage) return;
    const rejectAction = rejectToolUsageAction(
      [action.payload.toolCallId],
      action.payload.chatId,
      lastMessage.ftm_num,
      lastMessage.ftm_alt,
      lastMessage.ftm_prev_alt,
    );
    void listenerApi.dispatch(rejectAction);
  },
});

startListening({
  matcher: isAnyOf(updateConfig.match, setThemeMode.match),
  effect: (_action, listenerApi) => {
    const appearance = listenerApi.getState().config.themeProps.appearance;
    if (appearance === "light" && document.body.className !== "vscode-light") {
      document.body.className = "vscode-light";
    } else if (
      appearance === "dark" &&
      document.body.className !== "vscode-dark"
    ) {
      document.body.className = "vscode-dark";
    }
  },
});

// JB file refresh
// TBD: this could include diff messages to
startListening({
  actionCreator: threadMessagesSlice.actions.receiveThreadMessages,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (state.config.host !== "jetbrains") return;
    if (!isToolMessage(action.payload.news_payload_thread_message)) return;
    if (!window.postIntellijMessage) return;
    window.postIntellijMessage(ideForceReloadProjectTreeFiles());
  },
});
