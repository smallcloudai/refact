import type { RootState, AppDispatch } from "./store";
import {
  createListenerMiddleware,
  isAnyOf,
  isRejected,
} from "@reduxjs/toolkit";
import {
  newChatAction,
  restoreChat,
  newIntegrationChat,
  upsertToolCall,
  applyChatEvent,
  clearThreadPauseReasons,
  setThreadConfirmationStatus,
  setThreadPauseReasons,
  resetThreadImages,
  switchToThread,
  selectCurrentThreadId,
  ideToolRequired,
  saveTitle,
  setBoostReasoning,
  setIncludeProjectInfo,
  setContextTokensCap,
  setEnabledCheckpoints,
  setToolUse,
  setChatMode,
  setChatModel,
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
} from "../services/refact/commands";
import { pathApi } from "../services/refact/path";
import { pingApi } from "../services/refact/ping";
import {
  clearError,
  setError,
  setIsAuthError,
} from "../features/Errors/errorsSlice";
import { setThemeMode, updateConfig } from "../features/Config/configSlice";
import { nextTip } from "../features/TipOfTheDay";
import { telemetryApi } from "../services/refact/telemetry";
import { CONFIG_PATH_URL, FULL_PATH_URL } from "../services/refact/consts";
import {
  ideToolCallResponse,
  ideForceReloadProjectTreeFiles,
} from "../hooks/useEventBusForIDE";
import { upsertToolCallIntoHistory } from "../features/History/historySlice";
import { isToolMessage, isDiffMessage, modelsApi, providersApi } from "../services/refact";

const AUTH_ERROR_MESSAGE =
  "There is an issue with your API key. Check out your API Key or re-login";

export const listenerMiddleware = createListenerMiddleware();
const startListening = listenerMiddleware.startListening.withTypes<
  RootState,
  AppDispatch
>();

startListening({
  matcher: isAnyOf(
    (d: unknown): d is ReturnType<typeof newChatAction> =>
      newChatAction.match(d),
    (d: unknown): d is ReturnType<typeof restoreChat> => restoreChat.match(d),
  ),
  effect: (_action, listenerApi) => {
    const state = listenerApi.getState();
    const chatId = state.chat.current_thread_id;

    [
      statisticsApi.util.resetApiState(),
      toolsApi.util.resetApiState(),
      commandsApi.util.resetApiState(),
    ].forEach((api) => listenerApi.dispatch(api));

    listenerApi.dispatch(resetThreadImages({ id: chatId }));
    listenerApi.dispatch(clearThreadPauseReasons({ id: chatId }));
    listenerApi.dispatch(setThreadConfirmationStatus({ id: chatId, wasInteracted: false, confirmationStatus: true }));
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

// Telemetry for path API
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

startListening({
  actionCreator: ideToolCallResponse,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const chatId = action.payload.chatId;
    const { toolCallId, accepted } = action.payload;

    listenerApi.dispatch(upsertToolCallIntoHistory(action.payload));
    listenerApi.dispatch(upsertToolCall(action.payload));

    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "ide_tool_result",
        tool_call_id: toolCallId,
        content: accepted === true ? "Tool executed successfully" : "Tool execution rejected",
        tool_failed: accepted !== true,
      });
    } catch {
      // Silently ignore - backend may not support this command
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

// Auto-switch to thread when it needs confirmation (background chat support)
startListening({
  actionCreator: setThreadPauseReasons,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    const currentThreadId = selectCurrentThreadId(state);
    const threadIdNeedingConfirmation = action.payload.id;

    // If the thread needing confirmation is not the current one, switch to it
    if (threadIdNeedingConfirmation !== currentThreadId) {
      listenerApi.dispatch(switchToThread({ id: threadIdNeedingConfirmation }));
    }
  },
});

startListening({
  actionCreator: saveTitle,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = action.payload.id;
    const title = action.payload.title;
    const isTitleGenerated = action.payload.isTitleGenerated;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { title, is_title_generated: isTitleGenerated },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: applyChatEvent,
  effect: (action, listenerApi) => {
    const state = listenerApi.getState();
    if (state.config.host !== "jetbrains") return;
    if (!window.postIntellijMessage) return;

    const event = action.payload;
    if (event.type === "message_added") {
      const msg = event.message;
      if (isToolMessage(msg) || isDiffMessage(msg)) {
        window.postIntellijMessage(ideForceReloadProjectTreeFiles());
      }
    }
  },
});

startListening({
  actionCreator: applyChatEvent,
  effect: (action, listenerApi) => {
    const event = action.payload;
    if (event.type === "ide_tool_required") {
      listenerApi.dispatch(ideToolRequired({
        chatId: event.chat_id,
        toolCallId: event.tool_call_id,
        toolName: event.tool_name,
        args: event.args,
      }));
    }
  },
});

// Sync thread params to backend when changed via Redux actions
startListening({
  actionCreator: setBoostReasoning,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = action.payload.chatId;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { boost_reasoning: action.payload.value },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setIncludeProjectInfo,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = action.payload.chatId;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { include_project_info: action.payload.value },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setContextTokensCap,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = action.payload.chatId;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { context_tokens_cap: action.payload.value },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setEnabledCheckpoints,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = state.chat.current_thread_id;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { checkpoints_enabled: action.payload },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setToolUse,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = state.chat.current_thread_id;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { tool_use: action.payload },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setChatMode,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = state.chat.current_thread_id;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { mode: action.payload },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});

startListening({
  actionCreator: setChatModel,
  effect: async (action, listenerApi) => {
    const state = listenerApi.getState();
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    const chatId = state.chat.current_thread_id;

    if (!port || !chatId) return;

    try {
      const { sendChatCommand } = await import("../services/refact/chatCommands");
      await sendChatCommand(chatId, port, apiKey ?? undefined, {
        type: "set_params",
        patch: { model: action.payload },
      });
    } catch {
      // Silently ignore - backend may not support this command
    }
  },
});
