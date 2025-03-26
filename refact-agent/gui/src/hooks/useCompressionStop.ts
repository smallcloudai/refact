import { useEffect, useCallback, useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import {
  selectLastSentCompression,
  selectMessages,
  selectThreadPaused,
  setThreadPaused,
} from "../features/Chat";
import { takeFromEndWhile } from "../utils";
import { isUserMessage } from "../events";

export function useLastSentCompressionStop() {
  const dispatch = useAppDispatch();
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  const messages = useAppSelector(selectMessages);
  const stopped = useAppSelector(selectThreadPaused);
  useEffect(() => {
    if (lastSentCompression && lastSentCompression !== "absent" && !stopped) {
      dispatch(setThreadPaused(true));
    }
  }, [dispatch, lastSentCompression, stopped]);

  const messagesFromLastUserMessage = useMemo(() => {
    return takeFromEndWhile(messages, (message) => !isUserMessage(message))
      .length;
  }, [messages]);

  useEffect(() => {
    if (messagesFromLastUserMessage >= 40 && !stopped) {
      dispatch(setThreadPaused(true));
    }
  }, [dispatch, messagesFromLastUserMessage, stopped]);

  const resume = useCallback(() => {
    dispatch(setThreadPaused(false));
  }, [dispatch]);

  return { stopped, resume, strength: lastSentCompression };
}
