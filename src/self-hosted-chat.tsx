/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import ReactDOM from "react-dom/client";
import App from "./App.tsx";

function Chat(element: HTMLElement, lspUrl?: string) {
  ReactDOM.createRoot(element).render(<App lspUrl={lspUrl} />);
}

export default Chat;
