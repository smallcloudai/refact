import React from "react";
import { ChatHistory } from "../components/ChatHistory";
import { useEventBusForSidebar } from "../hooks";

export const HistoryList: React.FC = () => {
  const {
    history,
    onDeleteHistoryItem,
    onOpenChatInSIdeBar,
    onOpenChatInTab,
    onCreateNewChat,
  } = useEventBusForSidebar();

  return (
    <ChatHistory
      history={history}
      onDeleteHistoryItem={onDeleteHistoryItem}
      onHistoryItemClick={onOpenChatInSIdeBar}
      onOpenChatInTab={onOpenChatInTab}
      onCreateNewChat={onCreateNewChat}
    />
  );
};
