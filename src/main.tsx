/**
 * Only used by the dev server
 */

import { renderApp } from "./lib";

const element = document.getElementById("refact-chat");

if (element) {
  renderApp(element, {
    host: "web",
    features: { statistics: false, vecdb: true, ast: true },
  });
}
