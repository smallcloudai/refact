import { useCallback } from "react";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { isAbsolutePath } from "../utils/isAbsolutePath";
import { useAppDispatch } from "./useAppDispatch";
import { popBackTo } from "../features/Pages/pagesSlice";
import { useAppSelector } from "./useAppSelector";
import { selectIntegration } from "../features/Chat/Thread/selectors";
import { debugIntegrations } from "../debugConfig";

export function useGoToLink() {
  const dispatch = useAppDispatch();
  const { queryPathThenOpenFile } = useEventsBusForIDE();
  const maybeIntegration = useAppSelector(selectIntegration);

  const handleGoTo = useCallback(
    ({ goto }: { goto?: string }) => {
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
          debugIntegrations(`[DEBUG]: maybeIntegration: `, maybeIntegration);
          if (!maybeIntegration) {
            debugIntegrations(`[DEBUG]: integration data is not available.`);
            return;
          }
          dispatch(
            popBackTo({
              name: "integrations page",
              // projectPath: isFile ? payload : "",
              integrationName:
                !isFile && payload !== "DEFAULT"
                  ? payload
                  : maybeIntegration.name,
              integrationPath: isFile ? payload : maybeIntegration.path,
              projectPath: maybeIntegration.project,
              shouldIntermediatePageShowUp:
                payload !== "DEFAULT"
                  ? maybeIntegration.shouldIntermediatePageShowUp
                  : false,
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
      // maybeIntegration?.name,
      // maybeIntegration?.path,
      // maybeIntegration?.project,
      // maybeIntegration?.shouldIntermediatePageShowUp,
      maybeIntegration,
      queryPathThenOpenFile,
    ],
  );

  return { handleGoTo };
}
