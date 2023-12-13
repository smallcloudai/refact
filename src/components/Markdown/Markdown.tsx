import React from "react";
import ReactMarkdown from "react-markdown";

export const Markdown: React.FC<
  Pick<React.ComponentProps<typeof ReactMarkdown>, "children">
> = ({ children }) => {
  // TODO add code highlighting and use radix components
  // TODO: setup syntax highlighting
  return <ReactMarkdown>{children}</ReactMarkdown>;
};
