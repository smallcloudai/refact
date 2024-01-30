import React from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import { HistoryList } from "../../features/HistoryList.tsx";
import ReactDOM from "react-dom/client";
import { Theme } from "../../components/Theme";

export function renderHistoryList(element: HTMLElement, config: Config) {
  const List: React.FC<Config> = (config) => {
    return (
      <ConfigProvider config={config}>
        <Theme>
          <HistoryList />
        </Theme>
      </ConfigProvider>
    );
  };
  ReactDOM.createRoot(element).render(<List {...config} />);
}
