import React from "react";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { useChatHistory } from "../hooks/useChatHistory";

export const HistorySideBar: React.FC<{
  takingNotes: boolean;
  currentChatId: string;
  className?: string;
  style?: React.CSSProperties;
}> = ({ takingNotes, currentChatId, className, style }) => {
  const { history, restoreChatFromHistory, createNewChat, deleteChat } =
    useChatHistory();
  return (
    <Sidebar
      handleNavigation={() => ({})}
      takingNotes={takingNotes}
      history={history}
      onHistoryItemClick={restoreChatFromHistory}
      onCreateNewChat={createNewChat}
      onDeleteHistoryItem={deleteChat}
      currentChatId={currentChatId}
      className={className}
      style={style}
      handleLogout={() => {
        // TODO: handle logout
      }}
    />
  );
};
