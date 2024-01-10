import ReactDOM from "react-dom/client";
import { Theme } from "../components/Theme/Theme";
import { Chat } from "../features/Chat";
import { Config, ConfigProvider } from "../contexts/config-context";

const App: React.FC<Config> = (props) => (
  <ConfigProvider config={props}>
    <Theme>
      <Chat />
    </Theme>
  </ConfigProvider>
);

export const ChatForIDE = (element: HTMLElement, config: Config) => {
  ReactDOM.createRoot(element).render(<App {...config} />);
};
