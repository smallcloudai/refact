import React from "react";
import { Chat as ChatComponent } from "../../components/Chat";
import { useThreadMessageSubscription } from "../../hooks/useThreadMessageSubscription";
export const Chat: React.FC = () => {
  useThreadMessageSubscription();
  return <ChatComponent />;
};
