import { useEffect } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import { useConfig } from "./useConfig";
import { updateConfig } from "../features/Config/configSlice";
import { setFileInfo } from "../features/Chat/activeFile";
import { setSelectedSnippet } from "../features/Chat/selectedSnippet";
import { setCurrentProjectInfo } from "../features/Chat/currentProject";
import { newChatAction } from "../features/Chat/Thread/actions";
import {
  isPageInHistory,
  push,
  selectPages,
} from "../features/Pages/pagesSlice";
import { ideToolCallResponse } from "./useEventBusForIDE";

export function useEventBusForApp() {
  const config = useConfig();
  const dispatch = useAppDispatch();
  const pages = useAppSelector(selectPages);

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

      if (setCurrentProjectInfo.match(event.data)) {
        dispatch(setCurrentProjectInfo(event.data.payload));
      }

      if (ideToolCallResponse.match(event.data)) {
        dispatch(event.data);
      }

      // TODO: ideToolEditResponse.

      // TODO: active project
      // vscode workspace can be found with vscode.workspace.name
      // JB: project.name
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [config.host, dispatch, pages]);
}
