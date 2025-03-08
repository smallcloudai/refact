import { useEffect } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { subscribeToThreadMessagesThunk } from "../services/refact";
import { useAppSelector } from "./useAppSelector";
import { chatDbMessagesSliceSelectors } from "../features/ChatDB/chatDbMessagesSlice";

export function useThreadMessageSubscription() {
  const threadId = useAppSelector(chatDbMessagesSliceSelectors.selectThreadId);
  const dispatch = useAppDispatch();
  useEffect(() => {
    console.log("Subscribe to thread messages: " + threadId);
    const thunk = dispatch(subscribeToThreadMessagesThunk(threadId));
    return () => {
      try {
        thunk.catch(() => ({}));
        thunk.abort(`aborted: subscribeToThreadMessagesThunk(${threadId})`);
      } catch (e) {
        // no-op
      }
    };
  }, [dispatch, threadId]);
}
