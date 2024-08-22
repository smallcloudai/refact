import { useCallback } from "react";
import { usePostMessage } from ".";
import { EVENT_NAMES_FROM_SETUP } from "../events";

export function useOpenUrl() {
  const postMessage = usePostMessage();

  const openUrl = useCallback(
    (url: string) => {
      postMessage({
        type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL,
        payload: { url },
      });
    },
    [postMessage],
  );

  return openUrl;
}
