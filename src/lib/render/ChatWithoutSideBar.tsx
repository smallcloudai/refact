/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import React from "react";
import { Theme } from "../../components/Theme/index.ts";
import { Chat } from "../../features/Chat.tsx";

export const ChatWithOutSideBar: React.FC = () => {
  return (
    <Theme>
      <Chat />
    </Theme>
  );
};
