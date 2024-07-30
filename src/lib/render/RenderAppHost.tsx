import { StrictMode } from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context";
import { Theme } from "../../components/Theme/index.ts";
import { App } from "../../features/App";
import ReactDOM from "react-dom/client";
import "./web.css";
import { useEventBusForApp } from "../../hooks/useEventBusForApp.ts";

export function renderAppHost(element: HTMLElement, config: Config) {
  const AppWrapped: React.FC<Config> = (config) => {
    const newConfig = useEventBusForApp(config).config;
    return (
      <StrictMode>
        <ConfigProvider config={newConfig} key={JSON.stringify(newConfig)}>
          <Theme>
            <App />
          </Theme>
        </ConfigProvider>
      </StrictMode>
    );
  };
  ReactDOM.createRoot(element).render(<AppWrapped {...config} />);
}
