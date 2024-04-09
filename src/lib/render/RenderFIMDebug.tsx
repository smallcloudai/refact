import React from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import ReactDOM from "react-dom/client";
import { Theme } from "../../components/Theme";
import { FIMDebug } from "../../features/FIMDebug.tsx";

export function renderFIMDebug(element: HTMLElement, config: Config) {
  const FIMDebugApp: React.FC<Config> = (config) => {
    return (
      <ConfigProvider config={config}>
        <Theme>
          <FIMDebug />
        </Theme>
      </ConfigProvider>
    );
  };
  ReactDOM.createRoot(element).render(<FIMDebugApp {...config} />);
}
