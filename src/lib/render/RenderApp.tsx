import { StrictMode } from "react";
import { type Config } from "../../features/Config/configSlice";
import { App } from "../../features/App";
import ReactDOM from "react-dom/client";
import "./web.css";

export function renderApp(element: HTMLElement, config: Config) {
  const AppWrapped: React.FC<Config> = () => {
    return (
      <StrictMode>
        <App />
      </StrictMode>
    );
  };
  ReactDOM.createRoot(element).render(<AppWrapped {...config} />);
}
