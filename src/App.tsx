import "./App.css";
import "@radix-ui/themes/styles.css";
import { Theme, Flex } from "@radix-ui/themes";
import { Chat } from "./features/Chat";
import { useEventBusForHost } from "./hooks/useEventBusForHost";
import { HistorySideBar } from "./features/HistorySideBar";
import { PageWrapper } from "./components/PageWrapper";

function App() {
  const { history } = useEventBusForHost();
  return (
    <Theme>
      <Flex>
        <HistorySideBar history={history} />
        <PageWrapper>
          <Chat />
        </PageWrapper>
      </Flex>
    </Theme>
  );
}

export default App;
