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
  setIsWaitingForResponse,
  upsertToolCall,
  sendCurrentChatToLspAfterToolCallUpdate,
  chatResponse,
  chatError,
} from "../features/Chat/Thread";
import { statisticsApi } from "../services/refact/statistics";
import { integrationsApi } from "../services/refact/integrations";
import { dockerApi } from "../services/refact/docker";
import { capsApi, isCapsErrorResponse } from "../services/refact/caps";
import { promptsApi } from "../services/refact/prompts";
import { toolsApi } from "../services/refact/tools";
import {
  commandsApi,
  isDetailMessage,
  isDetailMessageWithErrorType,
} from "../services/refact/commands";
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
  resetConfirmationInteractedState,
  updateConfirmationAfterIdeToolUse,
} from "../features/ToolConfirmation/confirmationSlice";
import {
  ideToolCallResponse,
  ideForceReloadProjectTreeFiles,
} from "../hooks/useEventBusForIDE";
import { upsertToolCallIntoHistory } from "../features/History/historySlice";
import { isToolResponse, modelsApi, providersApi } from "../services/refact";

const AUTH_ERROR_MESSAGE =
  "There is an issue with your API key. Check out your API Key or re-login";

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
      // pingApi.util.resetApiState(),
      statisticsApi.util.resetApiState(),
      // capsApi.util.resetApiState(),
      // promptsApi.util.resetApiState(),
      toolsApi.util.resetApiState(),
      commandsApi.util.resetApiState(),
      resetAttachedImagesSlice(),
      resetConfirmationInteractedState(),
    ].forEach((api) => listenerApi.dispatch(api));

    listenerApi.dispatch(clearError());
  },
});

// TODO: think about better cache invalidation approach instead of listening for an action dispatching globally
startListening({
  matcher: isAnyOf((d: unknown): d is ReturnType<typeof newIntegrationChat> =>
    newIntegrationChat.match(d),
  ),
  effect: (_action, listenerApi) => {
    [integrationsApi.util.resetApiState()].forEach((api) =>
      listenerApi.dispatch(api),
    );
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
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isCapsErrorResponse(action.payload?.data)
          ? action.payload.data.detail
          : `fetching caps from lsp`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }
    if (
      toolsApi.endpoints.getTools.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `fetching tools from lsp`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }
    if (
      toolsApi.endpoints.checkForConfirmation.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail
          : `confirmation check from lsp`;

      listenerApi.dispatch(setError(message));
      listenerApi.dispatch(setIsAuthError(isAuthError));
    }
    if (
      promptsApi.endpoints.getPrompts.matchRejected(action) &&
      !action.meta.condition
    ) {
      const errorStatus = action.payload?.status;
      const isAuthError = errorStatus === 401;
      const message = isAuthError
        ? AUTH_ERROR_MESSAGE
        : isDetailMessage(action.payload?.data)
          ? action.payload.data.detail.split("\n").slice(0, 2).join("\n")
          : `fetching system prompts.`;

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

    if (
      chatAskQuestionThunk.rejected.match(action) &&
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
    // const toolResult = await toolsRequest.unwrap();
    // TODO: set mode to configure ? or infer it later
    // TODO: create a dedicated thunk for this.
    await listenerApi.dispatch(
      chatAskQuestionThunk({
        messages: state.chat.thread.messages,
        chatId: state.chat.thread.id,
        // tools: toolResult,
      }),
    );
  },
});

// Telemetry
startListening({
  matcher: isAnyOf(
    chatAskQuestionThunk.rejected.match,
    chatAskQuestionThunk.fulfilled.match,
    // give files api
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
    if (chatAskQuestionThunk.rejected.match(action) && !action.meta.condition) {
      const { chatId, mode } = action.meta.arg;
      const thread =
        chatId in state.chat.cache
          ? state.chat.cache[chatId]
          : state.chat.thread;
      const scope = `sendChat_${thread.model}_${mode}`;

      if (isDetailMessageWithErrorType(action.payload)) {
        const errorMessage = action.payload.detail;
        listenerApi.dispatch(
          action.payload.errorType === "GLOBAL"
            ? setError(errorMessage)
            : chatError({ id: chatId, message: errorMessage }),
        );
        const thunk = telemetryApi.endpoints.sendTelemetryChatEvent.initiate({
          scope,
          success: false,
          error_message: errorMessage,
        });
        void listenerApi.dispatch(thunk);
      }
    }

    if (chatAskQuestionThunk.fulfilled.match(action)) {
      const { chatId, mode } = action.meta.arg;
      const thread =
        chatId in state.chat.cache
          ? state.chat.cache[chatId]
          : state.chat.thread;
      const scope = `sendChat_${thread.model}_${mode}`;

      const thunk = telemetryApi.endpoints.sendTelemetryChatEvent.initiate({
        scope,
        success: true,
        error_message: "",
      });

      void listenerApi.dispatch(thunk);
    }

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

// Tool Call results from ide.
startListening({
  actionCreator: ideToolCallResponse,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();

    listenerApi.dispatch(upsertToolCallIntoHistory(action.payload));
    listenerApi.dispatch(upsertToolCall(action.payload));
    listenerApi.dispatch(updateConfirmationAfterIdeToolUse(action.payload));

    const pauseReasons = state.confirmation.pauseReasons.filter(
      (reason) => reason.tool_call_id !== action.payload.toolCallId,
    );

    if (pauseReasons.length === 0) {
      listenerApi.dispatch(resetConfirmationInteractedState());
      listenerApi.dispatch(setIsWaitingForResponse(false));
    }

    if (pauseReasons.length === 0 && action.payload.accepted) {
      void listenerApi.dispatch(
        sendCurrentChatToLspAfterToolCallUpdate({
          chatId: action.payload.chatId,
          toolCallId: action.payload.toolCallId,
        }),
      );
    }
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
  actionCreator: chatResponse,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (state.config.host !== "jetbrains") return;
    if (!isToolResponse(action.payload)) return;
    if (!window.postIntellijMessage) return;
    window.postIntellijMessage(ideForceReloadProjectTreeFiles());
  },
});
