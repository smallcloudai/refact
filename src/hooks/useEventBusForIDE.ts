import { useCallback } from "react";
import { createAction } from "@reduxjs/toolkit";
import { usePostMessage } from "./usePostMessage";
import type { ChatThread } from "../features/Chat/Thread/types";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  SetupHost,
} from "../events/setup";
import type { DiffPreviewResponse, PatchResult } from "../services/refact";

export const ideDiffPasteBackAction = createAction<string>("ide/diffPasteBack");

export const ideDiffPreviewAction = createAction<
  DiffPreviewResponse & { currentPin: string; allPins: string[] }
>("ide/diffPreview");

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

export const ideWriteResultsToFile = createAction<PatchResult[]>(
  "ide/writeResultsToFile",
);

export const ideChatPageChange = createAction<string>("ide/chatPageChange");
export const ideEscapeKeyPressed = createAction<string>("ide/escapeKeyPressed");

export const ideIsChatStreaming = createAction<boolean>("ide/isChatStreaming");
export const ideIsChatReady = createAction<boolean>("ide/isChatReady");

import { pathApi } from "../services/refact/path";

import { telemetryApi } from "../services/refact/telemetry";

export const useEventsBusForIDE = () => {
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
    (content: string) => {
      const action = ideDiffPasteBackAction(content);
      postMessage(action);
    },
    [postMessage],
  );

  const diffPreview = useCallback(
    (preview: DiffPreviewResponse, currentPin: string, allPins: string[]) => {
      postMessage(ideDiffPreviewAction({ ...preview, currentPin, allPins }));
    },
    [postMessage],
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

  const writeResultsToFile = useCallback(
    (results: PatchResult[]) => {
      const action = ideWriteResultsToFile(results);
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
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

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

  return {
    diffPasteBack,
    openSettings,
    newFile,
    openHotKeys,
    openFile,
    openChatInNewTab,
    setupHost,
    diffPreview,
    queryPathThenOpenFile,
    openCustomizationFile,
    openPrivacyFile,
    openBringYourOwnKeyFile,
    openIntegrationsFile,
    // canPaste,
    stopFileAnimation,
    startFileAnimation,
    writeResultsToFile,
    chatPageChange,
    escapeKeyPressed,
    setIsChatStreaming,
    setIsChatReady,
  };
};
