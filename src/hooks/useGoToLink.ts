import { useCallback } from "react";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { isAbsolutePath } from "../utils/isAbsolutePath";
import { useAppDispatch } from "./useAppDispatch";
import { popBackTo } from "../features/Pages/pagesSlice";
import { useAppSelector } from "./useAppSelector";
import { selectIntegration } from "../features/Chat/Thread/selectors";

export function useGoToLink() {
  const dispatch = useAppDispatch();
  const { queryPathThenOpenFile } = useEventsBusForIDE();
  const maybeIntegration = useAppSelector(selectIntegration);

  const handleGoTo = useCallback(
    (goto?: string) => {
      if (!goto) return;
      // TODO:  duplicated in smart links.
      const [action, payload] = goto.split(":");

      switch (action.toLowerCase()) {
        case "editor": {
          void queryPathThenOpenFile({ file_name: payload });
          return;
        }
        case "settings": {
          const isFile = isAbsolutePath(payload);
          dispatch(
            popBackTo({
              name: "integrations page",
              // projectPath: isFile ? payload : "",
              integrationName:
                !isFile && payload !== "DEFAULT"
                  ? payload
                  : maybeIntegration?.name,
              integrationPath: isFile ? payload : maybeIntegration?.path,
              projectPath: maybeIntegration?.project,
            }),
          );
          // TODO: open in the integrations
          return;
        }
        default: {
          // eslint-disable-next-line no-console
          console.log(`[DEBUG]: unexpected action, doing nothing`);
          return;
        }
      }
    },
    [
      dispatch,
      maybeIntegration?.name,
      maybeIntegration?.path,
      maybeIntegration?.project,
      queryPathThenOpenFile,
    ],
  );

  return { handleGoTo };
}
