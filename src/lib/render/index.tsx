/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import ReactDOM from "react-dom/client";
import type { Config } from "../../contexts/config-context";
import { Chat } from "./Chat";

export { renderHistoryList } from "./RenderHistoryList";
export { renderStatistic } from "./RenderStatistic";

export function render(element: HTMLElement, config: Config) {
  ReactDOM.createRoot(element).render(<Chat {...config} />);
}
