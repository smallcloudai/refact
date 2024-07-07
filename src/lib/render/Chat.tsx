import React, { StrictMode } from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import { ChatWithOutSideBar } from "./ChatWithoutSideBar.tsx";
import { App } from "../../features/App.tsx";
import { Theme } from "../../components/Theme/index.ts";

export const Chat: React.FC<Config> = (config) => {
  return (
    <StrictMode>
      <ConfigProvider config={config}>
        {config.host === "web" || config.dev ? (
          <Theme>
            <App />
          </Theme>
        ) : (
          <ChatWithOutSideBar />
        )}
      </ConfigProvider>
    </StrictMode>
  );
};
