import React from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import ReactDOM from "react-dom/client";
import { Theme } from "../../components/Theme";
import { Statistic } from "../../components/Statistic/Statistic";
import { useEventBusForChat } from "../../hooks";

export function renderStatistic(element: HTMLElement, config: Config) {
  const StatisticTab: React.FC<Config> = (config) => {
    const { backFromChat } = useEventBusForChat();

    return (
      <ConfigProvider config={config}>
        <Theme>
          <Statistic backFromChat={backFromChat} />
        </Theme>
      </ConfigProvider>
    );
  };
  ReactDOM.createRoot(element).render(<StatisticTab {...config} />);
}
