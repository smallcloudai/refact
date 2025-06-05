import { useCallback, useEffect, useMemo } from "react";
import { v4 as uuid } from "uuid";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectCurrentPage } from "../../features/Pages/pagesSlice";
import {
  messagesSub,
  createMessage,
} from "../../services/graphql/graphqlThunks";
import { FThreadMessageInput } from "../../../generated/documents";
import { selectThreadLeaf } from "../../features/ThreadMessages";

export function useMessageSubscription() {
  const dispatch = useAppDispatch();
  const ftId = useIdForThread();
  const leafMessage = useAppSelector(selectThreadLeaf);
  useEffect(() => {
    if (ftId.isNew) return;
    console.log("creating message sub");
    const thunk = dispatch(
      messagesSub({ ft_id: ftId.ft_id, want_deltas: true }),
    );
    return () => {
      console.log("removing message subscription");
      thunk.abort();
    };
  }, [dispatch, ftId]);

  // It'll need the parent node, and the info for the new node
  // What about images?
  const sendMessage = useCallback(
    (content: string) => {
      if (leafMessage === null) {
        // new thread
        return;
      }
      const input: FThreadMessageInput = {
        ftm_alt: leafMessage.ftm_alt, // increase when branching
        // ftm_app_specific: leafMessage.ftm_belongs_to_ft_id, // optional
        ftm_belongs_to_ft_id: leafMessage.ftm_belongs_to_ft_id, // ftId.ft_id,
        ftm_call_id: "",
        ftm_content: JSON.stringify(content),
        ftm_num: leafMessage.ftm_num + 1,
        ftm_prev_alt: leafMessage.ftm_alt, // optional
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_role: "user",
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
      };
      void dispatch(createMessage({ input }));
    },
    [dispatch, leafMessage],
  );

  return { sendMessage };
}

// TODO: id comes from the route or backend when creating a new thread
export const useIdForThread = () => {
  const route = useAppSelector(selectCurrentPage);

  const idInfo = useMemo(() => {
    if (route && "ft_id" in route && route.ft_id) {
      return { ft_id: route.ft_id, isNew: false };
    }
    return {
      ft_id: uuid(),
      isNew: true,
    };
  }, [route]);

  return idInfo;
};
