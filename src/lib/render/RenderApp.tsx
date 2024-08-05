import { StrictMode } from "react";
import { type Config } from "../../app/hooks";
import { Theme } from "../../components/Theme/index.ts";
import { App } from "../../features/App";
import ReactDOM from "react-dom/client";
import "./web.css";

export function renderApp(element: HTMLElement, config: Config) {
  const AppWrapped: React.FC<Config> = () => {
    return (
      <StrictMode>
        <Theme>
          <App />
        </Theme>
      </StrictMode>
    );
  };
  ReactDOM.createRoot(element).render(<AppWrapped {...config} />);
}
