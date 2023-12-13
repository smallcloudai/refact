import "./App.css";
import "@radix-ui/themes/styles.css";
import { Theme } from "@radix-ui/themes";
import { Chat } from "./features/Chat";
import { useEventBusForHost } from "./hooks/useEventBusForHost";


function App() {
  useEventBusForHost();
  return (
    <Theme>

      <Chat />
    </Theme>
  );
}

export default App;
