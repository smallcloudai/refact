import { useCallback } from "react";
import { v4 as uuidv4 } from "uuid";
import { selectThread } from "../features/Chat/Thread/selectors";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import {
  restoreChat,
  setSendImmediately,
} from "../features/Chat/Thread/actions";
import { saveChat } from "../features/History/historySlice";

export function useCompressChat() {
  const dispatch = useAppDispatch();
  const thread = useAppSelector(selectThread);
  const compressChat = useCallback(() => {
    const now = new Date().toISOString();
    const newId = uuidv4();
    const newThread = {
      ...thread,
      id: newId,
      createdAt: now,
      updatedAt: now,
      title: thread.title ?? "",
      read: false,
    };
    dispatch(saveChat(newThread));
    dispatch(restoreChat(newThread));
    dispatch(setSendImmediately(true));
  }, [dispatch, thread]);

  return { compressChat };
}
