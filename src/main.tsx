/**
 * Only used by the dev server
 */

import { ChatForWeb } from "./lib";

const element = document.getElementById("refact-chat");

if (element) {
  ChatForWeb(element, { host: "web", vecdb: false });
}
