import React from "react";
import type { Config } from "../Config/configSlice";
import { Chat as ChatComponent } from "../../components/Chat";
import { useAppSelector } from "../../hooks";
import { selectHasUncalledTools, selectMessages } from "./Thread";

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

  const unCalledTools = useAppSelector(selectHasUncalledTools);
  return (
    <ChatComponent
      style={style}
      host={host}
      tabbed={tabbed}
      backFromChat={backFromChat}
      unCalledTools={unCalledTools}
      maybeSendToSidebar={maybeSendToSideBar}
    />
  );
};
