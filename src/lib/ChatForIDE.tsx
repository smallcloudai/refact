import ReactDOM from "react-dom/client";
import { BaseTheme as Theme } from "../components/Theme/Theme";
import { Chat } from "../features/Chat";

const App: React.FC = () => (
  <Theme>
    <Chat />
  </Theme>
);

export const ChatForIDE = (element: HTMLElement) => {
  ReactDOM.createRoot(element).render(<App />);
};
