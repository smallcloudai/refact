import React from "react";
import { Sidebar } from "../components/Sidebar/Sidebar";
import type { ChatHistoryItem } from "../hooks/useChatHistory";

export const HistorySideBar: React.FC<{
  history: ChatHistoryItem[];
}> = ({ history }) => {
  return <Sidebar history={history} />;
};
