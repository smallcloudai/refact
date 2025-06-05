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
  selectThreadLeaf,
} from "../../features/ThreadMessages";

// function usecreateThreadWithMessage() {

// }

export function useMessageSubscription() {
  const dispatch = useAppDispatch();
  const leafMessage = useAppSelector(selectThreadLeaf);
  const isEmpty = useAppSelector(isThreadEmpty);
  const maybeFtId = useIdForThread();
  useEffect(() => {
    if (!maybeFtId) return;
    console.log("subscribing to message subscription");
    const thunk = dispatch(
      messagesSub({ ft_id: maybeFtId, want_deltas: true }),
    );
    return () => {
      console.log("removing message subscription");
      thunk.abort();
    };
  }, [dispatch, isEmpty, maybeFtId]);

  // It'll need the parent node, and the info for the new node
  // What about images?
  const sendMessage = useCallback(
    (content: string) => {
      console.log("Sending message");
      console.log({ content, leafMessage });
      if (leafMessage === null) {
        void dispatch(createThreadWithMessage({ ftm_content: content }));
        return;
      }
      const input: FThreadMessageInput = {
        ftm_alt: leafMessage.ftm_alt, // increase when branching
        // ftm_app_specific: leafMessage.ftm_belongs_to_ft_id, // optional
        ftm_belongs_to_ft_id: maybeFtId ?? leafMessage.ftm_belongs_to_ft_id, // ftId.ft_id,
        ftm_call_id: "",
        ftm_content: JSON.stringify(content),
        ftm_num: leafMessage.ftm_num + 1,
        ftm_prev_alt: leafMessage.ftm_alt, // optional
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_role: "user",
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
      };
      console.log("input", input);
      void dispatch(createMessage({ input }));
    },
    [dispatch, leafMessage, maybeFtId],
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
