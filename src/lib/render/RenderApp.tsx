import { StrictMode } from "react";
import { ConfigProvider, type Config } from "../../contexts/config-context";
import { Theme } from "../../components/Theme/index.ts";
import { App } from "../../features/App";
import ReactDOM from "react-dom/client";
import "./web.css";

export function renderApp(element: HTMLElement, config: Config) {
  const AppWrapped: React.FC<Config> = (config) => {
    return (
      <StrictMode>
        <ConfigProvider config={config}>
          <Theme>
            <App />
          </Theme>
        </ConfigProvider>
      </StrictMode>
    );
  };
  ReactDOM.createRoot(element).render(<AppWrapped {...config} />);
}
