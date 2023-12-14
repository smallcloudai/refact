import React from "react";
import ReactMarkdown from "react-markdown";
import SyntaxHighlighter from "react-syntax-highlighter";
import { Code } from "@radix-ui/themes";

export const Markdown: React.FC<
  Pick<React.ComponentProps<typeof ReactMarkdown>, "children">
> = ({ children }) => {
  return (
    <ReactMarkdown
      components={{
        code(props) {
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          const { children, className, node, color, ref, ...rest } = props;
          const match = /language-(\w+)/.exec(className ?? "");
          const withOutNewLines = String(children).replace(/\n$/, "");

          return match ? (
            <SyntaxHighlighter
              language={match[1]}
              // style={dark}
            >
              {withOutNewLines}
            </SyntaxHighlighter>
          ) : (
            <Code {...rest} className={className}>
              {children}
            </Code>
          );
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
};
