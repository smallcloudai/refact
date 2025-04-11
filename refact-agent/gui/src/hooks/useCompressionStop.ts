import { useEffect, useCallback, useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import {
  selectChatId,
  selectLastSentCompression,
  selectMessages,
  setIsNewChatSuggested,
  setIsNewChatSuggestionRejected,
  setPreventSend,
} from "../features/Chat";
import { takeFromEndWhile } from "../utils";
import { isUserMessage } from "../events";

export function useLastSentCompressionStop() {
  const dispatch = useAppDispatch();
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);

  const messagesFromLastUserMessage = useMemo(() => {
    return takeFromEndWhile(messages, (message) => !isUserMessage(message))
      .length;
  }, [messages]);

  useEffect(() => {
    if (
      lastSentCompression &&
      lastSentCompression !== "absent" &&
      messagesFromLastUserMessage >= 40
    ) {
      dispatch(setPreventSend({ id: chatId }));
      dispatch(setIsNewChatSuggested({ chatId, value: true }));
    }
  }, [chatId, dispatch, lastSentCompression, messagesFromLastUserMessage]);

  const resume = useCallback(() => {
    dispatch(setIsNewChatSuggestionRejected({ chatId, value: true }));
  }, [chatId, dispatch]);

  return { resume, strength: lastSentCompression };
}
