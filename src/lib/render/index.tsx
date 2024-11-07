/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import { renderApp } from "./RenderApp";
import { type Config } from "../../features/Config/configSlice";
import "./web.css";

export { renderApp } from "./RenderApp";

if (__REFACT_CHAT_VERSION__) {
  window.__REFACT_CHAT_VERSION__ = __REFACT_CHAT_VERSION__;
}

export function render(element: HTMLElement, config: Config) {
  renderApp(element, config);
}
