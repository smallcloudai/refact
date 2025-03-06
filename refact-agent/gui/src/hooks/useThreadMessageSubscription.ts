import { useEffect } from "react";
import { useThreadId } from "./useThreadId";
import { useAppDispatch } from "./useAppDispatch";
import { subscribeToThreadMessagesThunk } from "../services/refact";

export function useThreadMessageSubscription() {
  const threadId = useThreadId();
  const dispatch = useAppDispatch();
  useEffect(() => {
    const thunk = dispatch(subscribeToThreadMessagesThunk(threadId));
    return () => {
      try {
        thunk.abort(threadId);
      } catch (e) {
        // no-op
      }
    };
  }, [dispatch, threadId]);
}
