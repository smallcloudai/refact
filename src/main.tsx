/**
 * Only used by the dev server
 */

import { renderAppHost } from "./lib";

const element = document.getElementById("refact-chat");

if (element) {
  renderAppHost(element, {
    host: "web",
    features: { statistics: false, vecdb: true, ast: true },
  });
}
