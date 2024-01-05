import { Flex } from "@radix-ui/themes";
import { Chat } from "./features/Chat";
import { useEventBusForHost } from "./hooks/useEventBusForHost";
import { HistorySideBar } from "./features/HistorySideBar";
import { Theme } from "./components/Theme";
import "./App.css";

const App: React.FC<{
  lspUrl?: string;
}> = ({ lspUrl }: { lspUrl?: string }) => {
  useEventBusForHost(lspUrl);
  // TODO: maybe make light and dark mode optional
  return (
    <Theme>
      <Flex>
        <HistorySideBar />
        {/* <PageWrapper> */}
        <Chat style={{ maxWidth: "calc(100vw - 260px)" }} />
        {/* </PageWrapper> */}
      </Flex>
    </Theme>
  );
};

export default App;
