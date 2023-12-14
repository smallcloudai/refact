import React from "react";
import { Sidebar } from "../components/Sidebar/Sidebar";
import { useChatHistory } from "../hooks/useChatHistory";

export const HistorySideBar: React.FC = () => {
  const { history, restoreChatFromHistory } = useChatHistory();
  return <Sidebar history={history} onHistoryItemClick={restoreChatFromHistory} />;
};
