import { useEffect } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import { useConfig } from "./useConfig";
import { updateConfig } from "../features/Config/configSlice";
import { setFileInfo } from "../features/Chat/activeFile";
import { setSelectedSnippet } from "../features/Chat/selectedSnippet";
import { newChatAction } from "../features/Chat/Thread/actions";
import {
  isPageInHistory,
  push,
  selectPages,
} from "../features/Pages/pagesSlice";
import { diffApi, resetDiffApi } from "../services/refact/diffs";
import { usePatchActions } from "./usePatchActions";
import { showPatchTicket } from "../events";

export function useEventBusForApp() {
  const config = useConfig();
  const dispatch = useAppDispatch();
  const pages = useAppSelector(selectPages);
  const { handleShow } = usePatchActions();

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (updateConfig.match(event.data)) {
        dispatch(updateConfig(event.data.payload));
      }

      if (setFileInfo.match(event.data)) {
        dispatch(setFileInfo(event.data.payload));
      }

      if (setSelectedSnippet.match(event.data)) {
        dispatch(setSelectedSnippet(event.data.payload));
      }

      if (newChatAction.match(event.data)) {
        if (!isPageInHistory({ pages }, "chat")) {
          dispatch(push({ name: "chat" }));
        }
        dispatch(newChatAction(event.data.payload));
      }

      if (resetDiffApi.match(event.data)) {
        dispatch(diffApi.util.resetApiState());
      }

      if (showPatchTicket.match(event.data)) {
        handleShow(event.data.payload);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [config.host, dispatch, handleShow, pages]);
}
