import { useEffect } from "react";
import { useThreadId } from "./useThreadId";
import { useAppDispatch } from "./useAppDispatch";
import { subscribeToThreadMessagesThunk } from "../services/refact";
// import { useAppSelector } from "./useAppSelector";
// import { chatDbSelectors } from "../features/ChatDB/chatDbSlice";

export function useThreadMessageSubscription() {
  // looks like we need to create the thread before subscribing.
  const threadId = useThreadId();
  const dispatch = useAppDispatch();
  useEffect(() => {
    const thunk = dispatch(subscribeToThreadMessagesThunk(threadId));
    return () => {
      try {
        thunk.abort("useThreadMessageSubscription unmounted");
      } catch (e) {
        // no-op
      }
    };
  }, [dispatch, threadId]);
}
