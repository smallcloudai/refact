import React from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import { ChatWithSideBar } from "./ChatWithSideBar.tsx";
import { ChatWithOutSideBar } from "./ChatWithoutSideBar.tsx";

export const Chat: React.FC<Config> = (config) => {
  return (
    <ConfigProvider config={config}>
      {config.host === "web" || config.dev ? (
        <ChatWithSideBar />
      ) : (
        <ChatWithOutSideBar />
      )}
    </ConfigProvider>
  );
};
