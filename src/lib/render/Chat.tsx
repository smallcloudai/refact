import React, { StrictMode } from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import { ChatWithOutSideBar } from "./ChatWithoutSideBar.tsx";
import "./web.css";
import { ChatWithSideBar } from "./ChatWithSideBar.tsx";

export const Chat: React.FC<Config> = (config) => {
  return (
    <StrictMode>
      <ConfigProvider config={config}>
        {config.host === "web" || config.dev ? (
          <ChatWithSideBar />
        ) : (
          <ChatWithOutSideBar />
        )}
      </ConfigProvider>
    </StrictMode>
  );
};
