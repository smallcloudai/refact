import React from "react";
import { Sidebar } from "../components/Sidebar/Sidebar";

// not used anywhere :/
export const HistorySideBar: React.FC<{
  takingNotes: boolean;
  // currentChatId: string;
  className?: string;
  style?: React.CSSProperties;
}> = ({ takingNotes, className, style }) => {
  return (
    <Sidebar
      handleNavigation={() => ({})}
      takingNotes={takingNotes}
      // history={history}
      // onHistoryItemClick={restoreChatFromHistory}
      // onCreateNewChat={createNewChat}
      // onDeleteHistoryItem={deleteChat}
      // currentChatId={currentChatId}
      className={className}
      style={style}
      handleLogout={() => {
        // TODO: handle logout
      }}
    />
  );
};
