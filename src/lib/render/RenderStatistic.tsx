import React from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context.tsx";
import ReactDOM from "react-dom/client";
import { Theme } from "../../components/Theme";
import { Statistic } from "../../components/Statistic/Statistic";
import { useEventBusForStatistic } from "../../hooks";

export function renderStatistic(element: HTMLElement, config: Config) {
  const StatisticTab: React.FC<Config> = (config) => {
    const { backFromStatistic } = useEventBusForStatistic();

    return (
      <ConfigProvider config={config}>
        <Theme>
          <Statistic backFromStatistic={backFromStatistic} />
        </Theme>
      </ConfigProvider>
    );
  };
  ReactDOM.createRoot(element).render(<StatisticTab {...config} />);
}
