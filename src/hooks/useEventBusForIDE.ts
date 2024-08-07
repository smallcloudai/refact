import { useCallback } from "react";
import { createAction } from "@reduxjs/toolkit";
import { usePostMessage } from "./usePostMessage";
// import { useAppSelector } from "../app/hooks";
export const ideDiffPasteBackAction = createAction<string>("ide/diffPasteBack");
export const ideOpenSettingsAction = createAction("ide/openSettings");
export const ideNewFileAction = createAction("ide/newFile");
export const ideOpenHotKeys = createAction("ide/openHotKeys");
// TODO: open file

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

  const newFile = useCallback(() => {
    const action = ideNewFileAction();
    postMessage(action);
  }, [postMessage]);

  const openHotKeys = useCallback(() => {
    const action = ideOpenHotKeys();
    postMessage(action);
  }, [postMessage]);

  return {
    diffPasteBack,
    openSettings,
    newFile,
    openHotKeys,
    // canPaste,
  };
};
