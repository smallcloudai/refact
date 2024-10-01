import { useCallback } from "react";
import { createAction } from "@reduxjs/toolkit";
import { usePostMessage } from "./usePostMessage";
import type { ChatThread } from "../features/Chat/Thread/types";
import {
  EVENT_NAMES_FROM_SETUP,
  HostSettings,
  SetupHost,
} from "../events/setup";
import type { DiffPreviewResponse } from "../services/refact";
export const ideDiffPasteBackAction = createAction<string>("ide/diffPasteBack");
export const ideDiffPreviewAction =
  createAction<DiffPreviewResponse>("ide/diffPreview");
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

import { pathApi } from "../services/refact/path";

export const useEventsBusForIDE = () => {
  const postMessage = usePostMessage();
  // const canPaste = useAppSelector((state) => state.active_file.can_paste);

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
    (preview: DiffPreviewResponse) => {
      postMessage(ideDiffPreviewAction(preview));
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

  const [getCustomizationPath] = pathApi.useLazyCustomizationPathQuery();
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
      }
    },
    [postMessage],
  );

  const openCustomizationFile = () =>
    openFileFromPathQuery(getCustomizationPath);
  const openPrivacyFile = () => openFileFromPathQuery(getPrivacyPath);
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
    // canPaste,
  };
};
