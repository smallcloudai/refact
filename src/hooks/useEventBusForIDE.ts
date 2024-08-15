import { useCallback } from "react";
import { createAction } from "@reduxjs/toolkit";
import { usePostMessage } from "./usePostMessage";
import type { ChatThread } from "../features/Chat";
// import { useAppSelector } from "../app/hooks";
export const ideDiffPasteBackAction = createAction<string>("ide/diffPasteBack");
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

export const useEventsBusForIDE = () => {
  const postMessage = usePostMessage();
  // const canPaste = useAppSelector((state) => state.active_file.can_paste);

  const diffPasteBack = useCallback(
    (content: string) => {
      const action = ideDiffPasteBackAction(content);
      postMessage(action);
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

  const openChatInNewTab = useCallback(
    (thread: ChatThread) => {
      const action = ideOpenChatInNewTab(thread);
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
    // canPaste,
  };
};
