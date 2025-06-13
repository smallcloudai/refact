import { useCallback, useEffect, useMemo } from "react";

import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectCurrentPage } from "../../features/Pages/pagesSlice";
import {
  messagesSub,
  createMessage,
  createThreadWithMessage,
} from "../../services/graphql/graphqlThunks";
import { FThreadMessageInput } from "../../../generated/documents";
import {
  isThreadEmpty,
  selectThreadId,
  selectThreadEnd,
  selectAppSpecific,
} from "../../features/ThreadMessages";

// function usecreateThreadWithMessage() {

// }

export function useMessageSubscription() {
  const dispatch = useAppDispatch();
  const leafMessage = useAppSelector(selectThreadEnd, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const isEmpty = useAppSelector(isThreadEmpty, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const maybeFtId = useIdForThread();
  const appSpecific = useAppSelector(selectAppSpecific, {
    devModeChecks: { stabilityCheck: "never" },
  });
  useEffect(() => {
    if (!maybeFtId) return;
    const thunk = dispatch(
      messagesSub({ ft_id: maybeFtId, want_deltas: true }),
    );
    return () => {
      thunk.abort();
    };
  }, [dispatch, isEmpty, maybeFtId]);

  // It'll need the parent node, and the info for the new node
  // What about images?
  const sendMessage = useCallback(
    (content: string) => {
      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void dispatch(createThreadWithMessage({ ftm_content: content }));
        return;
      }
      const input: FThreadMessageInput = {
        ftm_alt: leafMessage.endAlt,
        ftm_app_specific: JSON.stringify(appSpecific),
        ftm_belongs_to_ft_id: maybeFtId ?? "", // ftId.ft_id,
        ftm_call_id: "",
        ftm_content: JSON.stringify(content),
        ftm_num: leafMessage.endNumber + 1,
        ftm_prev_alt: leafMessage.endPrevAlt,
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_role: "user",
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
      };
      // TODO: this will need more info
      void dispatch(createMessage({ input }));
    },
    [appSpecific, dispatch, leafMessage, maybeFtId],
  );

  return { sendMessage };
}

// TODO: id comes from the route or backend when creating a new thread
export const useIdForThread = () => {
  const route = useAppSelector(selectCurrentPage);
  const ftId = useAppSelector(selectThreadId);

  const idInfo = useMemo(() => {
    if (ftId) return ftId;
    if (route && "ft_id" in route && route.ft_id) {
      return route.ft_id;
    }
    return null;
  }, [route, ftId]);

  return idInfo;
};
