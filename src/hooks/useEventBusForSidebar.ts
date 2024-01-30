import { useEffect, useState } from "react";
import type { ChatHistoryItem } from "./useChatHistory";
import {
  EVENT_NAMES_FROM_SIDE_BAR,
  isReceiveChatHistory,
} from "../events/sidebar";
import { usePostMessage } from "./usePostMessage";

export function useEventBusForSidebar() {
  const [history, setHistory] = useState<ChatHistoryItem[]>([]);

  const postMessage = usePostMessage();

  useEffect(() => {
    function requestHistory() {
      postMessage({ type: EVENT_NAMES_FROM_SIDE_BAR.REQUEST_CHAT_HISTORY });
    }
    const listener = (event: MessageEvent) => {
      if (isReceiveChatHistory(event.data)) {
        setHistory(event.data.payload);
      }
    };

    window.addEventListener("message", listener);
    requestHistory();
    return () => window.removeEventListener("message", listener);
  }, [postMessage]);

  const onOpenChatInSIdeBar = (id: string) => {
    postMessage({
      type: EVENT_NAMES_FROM_SIDE_BAR.OPEN_CHAT_IN_SIDEBAR,
      payload: { id },
    });
  };

  const onDeleteHistoryItem = (id: string) => {
    postMessage({
      type: EVENT_NAMES_FROM_SIDE_BAR.DELETE_HISTORY_ITEM,
      payload: { id },
    });
  };

  const onOpenChatInTab = (id: string) => {
    postMessage({
      type: EVENT_NAMES_FROM_SIDE_BAR.OPEN_IN_CHAT_IN_TAB,
      payload: { id },
    });
  };

  const onCreateNewChat = () => {
    postMessage({
      type: EVENT_NAMES_FROM_SIDE_BAR.CREATE_NEW_CHAT,
    });
  };

  return {
    history,
    onOpenChatInSIdeBar,
    onDeleteHistoryItem,
    onOpenChatInTab,
    onCreateNewChat,
  };
}
