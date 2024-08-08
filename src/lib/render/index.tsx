/**
 * Component for use with the self hosted service https://github.com/smallcloudai/refact
 */
import { renderApp } from "./RenderApp";
import { type Config } from "../../features/Config/reducer";
import "./web.css";

export { renderApp } from "./RenderApp";

export function render(element: HTMLElement, config: Config) {
  renderApp(element, config);
}
