import React from "react";
import { ChatHistory } from "../components/ChatHistory";
import { useEventBusForSidebar } from "../hooks";

// TODO: delete this
export const HistoryList: React.FC = () => {
  const {
    // history,
    onDeleteHistoryItem,
    onOpenChatInSIdeBar,
    onOpenChatInTab,
  } = useEventBusForSidebar();

  return (
    <ChatHistory
      history={[]}
      onDeleteHistoryItem={onDeleteHistoryItem}
      onHistoryItemClick={(thread) => onOpenChatInSIdeBar(thread.id)}
      onOpenChatInTab={onOpenChatInTab}
    />
  );
};
