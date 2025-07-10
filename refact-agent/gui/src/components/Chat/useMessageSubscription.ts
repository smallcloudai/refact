import { useEffect } from "react";
import { useAppDispatch, useIdForThread } from "../../hooks";
import { messagesSub } from "../../services/graphql/graphqlThunks";

export function useMessageSubscription() {
  const dispatch = useAppDispatch();

  const maybeFtId = useIdForThread();

  useEffect(() => {
    if (!maybeFtId) return;
    const thunk = dispatch(
      messagesSub({ ft_id: maybeFtId, want_deltas: true }),
    );
    return () => {
      thunk.abort();
    };
  }, [dispatch, maybeFtId]);
}
