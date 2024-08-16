import { useEffect } from "react";
import { useAppDispatch, useConfig } from "../app/hooks";
import { updateConfig } from "../features/Config/configSlice";
import { setFileInfo } from "../features/Chat/activeFile";
import { setSelectedSnippet } from "../features/Chat/selectedSnippet";

export function useEventBusForApp() {
  const config = useConfig();
  const dispatch = useAppDispatch();

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (updateConfig.match(event.data)) {
        dispatch(updateConfig(event.data.payload));
      }

      if (setFileInfo.match(event.data)) {
        dispatch(setFileInfo(event.data.payload));
      }

      if (setSelectedSnippet.match(event.data)) {
        dispatch(setSelectedSnippet(event.data.payload));
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [config.host, dispatch]);
}
