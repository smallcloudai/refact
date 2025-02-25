import { useCallback } from "react";
import { createAction } from "@reduxjs/toolkit";
import { usePostMessage } from "./usePostMessage";
import type { ChatThread } from "../features/Chat/Thread/types";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  SetupHost,
} from "../events/setup";
import { pathApi } from "../services/refact/path";

import { telemetryApi } from "../services/refact/telemetry";
import { ToolEditResult } from "../services/refact";
import { TextDocToolCall } from "../components/Tools/types";

export const ideDiffPasteBackAction = createAction<{
  content: string;
  chatId?: string;
  toolCallId?: string;
}>("ide/diffPasteBack");

export const ideOpenSettingsAction = createAction("ide/openSettings");

export const ideNewFileAction = createAction<string>("ide/newFile");

export const ideOpenHotKeys = createAction("ide/openHotKeys");

export type OpenFilePayload = {
  file_name: string;
  line?: number;
};
export const ideOpenFile = createAction<OpenFilePayload>("ide/openFile");

export const ideOpenChatInNewTab = createAction<ChatThread>(
  "ide/openChatInNewTab",
);

export const ideAnimateFileStart = createAction<string>(
  "ide/animateFile/start",
);

export const ideAnimateFileStop = createAction<string>("ide/animateFile/stop");

export const ideChatPageChange = createAction<string>("ide/chatPageChange");
export const ideEscapeKeyPressed = createAction<string>("ide/escapeKeyPressed");

export const ideIsChatStreaming = createAction<boolean>("ide/isChatStreaming");
export const ideIsChatReady = createAction<boolean>("ide/isChatReady");

export const ideToolCall = createAction<{
  toolCall: TextDocToolCall;
  chatId: string;
  edit: ToolEditResult;
}>("ide/toolEdit");

export const ideToolCallResponse = createAction<{
  toolCallId: string;
  chatId: string;
  accepted: boolean | "indeterminate";
}>("ide/toolEditResponse");

export const useEventsBusForIDE = () => {
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();
  const postMessage = usePostMessage();
  // const canPaste = useAppSelector((state) => state.active_file.can_paste);

  const startFileAnimation = useCallback(
    (fileName: string) => {
      const action = ideAnimateFileStart(fileName);
      postMessage(action);
    },
    [postMessage],
  );

  const stopFileAnimation = useCallback(
    (fileName: string) => {
      const action = ideAnimateFileStop(fileName);
      postMessage(action);
    },
    [postMessage],
  );

  const setupHost = useCallback(
    (host: HostSettings) => {
      const setupHost: SetupHost = {
        type: EVENT_NAMES_FROM_SETUP.SETUP_HOST,
        payload: {
          host,
        },
      };

      postMessage(setupHost);
    },
    [postMessage],
  );

  const diffPasteBack = useCallback(
    (content: string, chatId?: string, toolCallId?: string) => {
      const action = ideDiffPasteBackAction({ content, chatId, toolCallId });
      postMessage(action);
      void sendTelemetryEvent({
        scope: `replaceSelection`,
        success: true,
        error_message: "",
      });
    },
    [postMessage, sendTelemetryEvent],
  );

  const openSettings = useCallback(() => {
    const action = ideOpenSettingsAction();
    postMessage(action);
  }, [postMessage]);

  const newFile = useCallback(
    (content: string) => {
      const action = ideNewFileAction(content);
      postMessage(action);
    },
    [postMessage],
  );

  const openHotKeys = useCallback(() => {
    const action = ideOpenHotKeys();
    postMessage(action);
  }, [postMessage]);

  const openFile = useCallback(
    (file: OpenFilePayload) => {
      const action = ideOpenFile(file);
      postMessage(action);
    },
    [postMessage],
  );

  const [getFullPath, _] = pathApi.useLazyGetFullPathQuery();

  const queryPathThenOpenFile = useCallback(
    async (file: OpenFilePayload) => {
      const res = await getFullPath(file.file_name).unwrap();
      const file_name = res ?? file.file_name;
      const action = ideOpenFile({ file_name, line: file.line });
      postMessage(action);
    },
    [getFullPath, postMessage],
  );

  const openChatInNewTab = useCallback(
    (thread: ChatThread) => {
      const action = ideOpenChatInNewTab(thread);
      postMessage(action);
    },
    [postMessage],
  );

  const chatPageChange = useCallback(
    (page: string) => {
      const action = ideChatPageChange(page);
      postMessage(action);
    },
    [postMessage],
  );

  const escapeKeyPressed = useCallback(
    (mode: string) => {
      const action = ideEscapeKeyPressed(mode);
      postMessage(action);
    },
    [postMessage],
  );

  const setIsChatStreaming = useCallback(
    (state: boolean) => {
      const action = ideIsChatStreaming(state);
      postMessage(action);
    },
    [postMessage],
  );

  const setIsChatReady = useCallback(
    (state: boolean) => {
      const action = ideIsChatReady(state);
      postMessage(action);
    },
    [postMessage],
  );

  const [getCustomizationPath] = pathApi.useLazyCustomizationPathQuery();
  const [getIntegrationsPath] = pathApi.useLazyIntegrationsPathQuery();
  const [getPrivacyPath] = pathApi.useLazyPrivacyPathQuery();
  const [getBringYourOwnKeyPath] = pathApi.useLazyBringYourOwnKeyPathQuery();

  // Creating a generic function to trigger different queries from RTK Query (to avoid duplicative code)
  const openFileFromPathQuery = useCallback(
    async (
      getPathQuery: (arg: undefined) => {
        unwrap: () => Promise<string | undefined>;
      },
    ) => {
      const res = await getPathQuery(undefined).unwrap();

      if (res) {
        const action = ideOpenFile({ file_name: res });
        postMessage(action);
        const res_split = res.split("/");
        void sendTelemetryEvent({
          scope: `ideOpenFile/${res_split[res_split.length - 1]}`,
          success: true,
          error_message: "",
        });
      } else {
        void sendTelemetryEvent({
          scope: `ideOpenFile`,
          success: false,
          error_message: res?.toString() ?? "path is not found",
        });
      }
    },
    [postMessage, sendTelemetryEvent],
  );

  const openCustomizationFile = () =>
    openFileFromPathQuery(getCustomizationPath);

  const openPrivacyFile = () => openFileFromPathQuery(getPrivacyPath);
  const openIntegrationsFile = () => openFileFromPathQuery(getIntegrationsPath);

  const openBringYourOwnKeyFile = () =>
    openFileFromPathQuery(getBringYourOwnKeyPath);

  const sendToolCallToIde = useCallback(
    (toolCall: TextDocToolCall, edit: ToolEditResult, chatId: string) => {
      const action = ideToolCall({ toolCall, edit, chatId });
      postMessage(action);
    },
    [postMessage],
  );

  return {
    diffPasteBack,
    openSettings,
    newFile,
    openHotKeys,
    openFile,
    openChatInNewTab,
    setupHost,
    queryPathThenOpenFile,
    openCustomizationFile,
    openPrivacyFile,
    openBringYourOwnKeyFile,
    openIntegrationsFile,
    stopFileAnimation,
    startFileAnimation,
    chatPageChange,
    escapeKeyPressed,
    setIsChatStreaming,
    setIsChatReady,
    sendToolCallToIde,
  };
};
