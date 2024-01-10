/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import ReactDOM from "react-dom/client";
import { ConfigProvider, type Config } from "../contexts/config-context.tsx";
import { useEventBusForHost } from "../hooks/index.ts";
import { Theme } from "../components/Theme";
import { Flex } from "@radix-ui/themes";
import { HistorySideBar } from "../features/HistorySideBar.tsx";
import { Chat } from "../features/Chat.tsx";
import "./web.css";

const ChatWithSideBar = () => {
  useEventBusForHost();
  return (
    <Theme>
      <Flex>
        <HistorySideBar />
        <Chat style={{ maxWidth: "calc(100vw - 260px)" }} />
      </Flex>
    </Theme>
  );
};

const App: React.FC<Config> = (config) => {
  return (
    <ConfigProvider config={config}>
      <ChatWithSideBar />
    </ConfigProvider>
  );
};

export function ChatForWeb(element: HTMLElement, config: Config) {
  ReactDOM.createRoot(element).render(<App {...config} />);
}
