import React from "react";
export { type CommandMarkdownProps, type MarkdownProps } from "./Markdown";

import {
  CommandMarkdown as _CommandMarkdown,
  Markdown as _Markdown,
} from "./Markdown";

export const CommandMarkdown = React.memo(_CommandMarkdown);
export const Markdown = React.memo(_Markdown);
