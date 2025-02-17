import React from "react";
export {
  type CommandMarkdownProps,
  type ResultMarkdownProps,
  type MarkdownProps,
} from "./Markdown";

import {
  CommandMarkdown as _CommandMarkdown,
  ResultMarkdown as _ResultMarkdown,
  Markdown as _Markdown,
} from "./Markdown";

export const CommandMarkdown = React.memo(_CommandMarkdown);
export const ResultMarkdown = React.memo(_Markdown);
export const Markdown = React.memo(_Markdown);
