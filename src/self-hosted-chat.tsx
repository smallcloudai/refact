/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import ReactDOM from "react-dom/client";
import App from "./App.tsx";

/**
 *
 * @param element element to add the chat to
 * @param lspUrl where to send requests for the lsp or lsp-proxy
 */
function Chat(element: HTMLElement, lspUrl?: string) {
  ReactDOM.createRoot(element).render(<App lspUrl={lspUrl} />);
}

export default Chat;
