import React from "react";
import type { Config } from "../Config/configSlice";
import { Chat as ChatComponent } from "../../components/Chat";
import { useAppSelector } from "../../hooks";
import { selectBranchHasUncalledTools } from "../ThreadMessages";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  style?: React.CSSProperties;
  backFromChat: () => void;
};

export const Chat: React.FC<ChatProps> = ({
  style,
  backFromChat,
  host,
  tabbed,
}) => {
  const sendToSideBar = () => {
    // TODO:
  };

  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

  return (
    <ChatComponent
      style={style}
      host={host}
      tabbed={tabbed}
      backFromChat={backFromChat}
      maybeSendToSidebar={maybeSendToSideBar}
    />
  );
};
