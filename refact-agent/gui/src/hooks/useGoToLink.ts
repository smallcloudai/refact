import { useCallback } from "react";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { useAppDispatch } from "./useAppDispatch";
import { popBackTo, push } from "../features/Pages/pagesSlice";

export function useGoToLink() {
  const dispatch = useAppDispatch();
  const { queryPathThenOpenFile } = useEventsBusForIDE();
  // const maybeIntegration = useAppSelector(selectIntegration);

  const handleGoTo = useCallback(
    ({ goto }: { goto?: string }) => {
      if (!goto) return;
      // TODO:  duplicated in smart links.
      const [action, ...payloadParts] = goto.split(":");
      const payload = payloadParts.join(":");
      switch (action.toLowerCase()) {
        case "editor": {
          void queryPathThenOpenFile({ file_path: payload });
          return;
        }
        // case "settings": {
        //   const isFile = isAbsolutePath(payload);
        //   debugIntegrations(`[DEBUG]: maybeIntegration: `, maybeIntegration);
        //   if (!maybeIntegration) {
        //     debugIntegrations(`[DEBUG]: integration data is not available.`);
        //     return;
        //   }
        //   dispatch(
        //     popBackTo({
        //       name: "integrations page",
        //       // projectPath: isFile ? payload : "",
        //       integrationName:
        //         !isFile && payload !== "DEFAULT"
        //           ? payload
        //           : maybeIntegration.name,
        //       integrationPath: isFile ? payload : maybeIntegration.path,
        //       projectPath: maybeIntegration.project,
        //       shouldIntermediatePageShowUp:
        //         payload !== "DEFAULT"
        //           ? maybeIntegration.shouldIntermediatePageShowUp
        //           : false,
        //       wasOpenedThroughChat: true,
        //     }),
        //   );
        //   // TODO: open in the integrations
        //   return;
        // }

        case "newchat": {
          dispatch(popBackTo({ name: "history" }));
          dispatch(push({ name: "chat" }));
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
      // maybeIntegration,
      queryPathThenOpenFile,
    ],
  );

  return { handleGoTo };
}
