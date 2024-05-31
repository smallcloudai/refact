import React from "react";
import ReactMarkdown from "react-markdown";

import styles from "./CommandLine.module.css";

import SyntaxHighlighter from "react-syntax-highlighter";
import { Code } from "@radix-ui/themes";
import classNames from "classnames";
import type { Element } from "hast";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";

const CodeBlock: React.FC<
  React.JSX.IntrinsicElements["code"] & { node?: Element | undefined }
> = ({ children, className, color: _color, ref: _ref, node: _node }) => {
  const match = /language-(\w+)/.exec(className ?? "");
  const textWithOutTrailingNewLine = String(children).replace(/\n$/, "");

  const language: string = match && match.length > 0 ? match[1] : "text";
  return (
    <SyntaxHighlighter
      style={hljsStyle}
      className={className}
      CodeTag={(props) => (
        <Code {...props} size="2" className={classNames(styles.code)} />
      )}
      PreTag={(props) => <pre {...props} className={classNames(styles.pre)} />}
      language={language}
      // useInlineStyles={false}
    >
      {textWithOutTrailingNewLine.trim()}
    </SyntaxHighlighter>
  );
};

export const Markdown: React.FC<{ children: string; className?: string }> = ({
  children,
  className,
}) => {
  return (
    <ReactMarkdown
      className={classNames(styles.markdown, className)}
      components={{
        code(props) {
          return <CodeBlock {...props} />;
        },
        p(props) {
          return <CodeBlock {...props} />;
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
};
