import React from "react";
import ReactMarkdown from "react-markdown";

import styles from "./CommandLine.module.css";

import SyntaxHighlighter from "react-syntax-highlighter";
import { Code } from "@radix-ui/themes";
import classNames from "classnames";
import type { Element } from "hast";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import resultStyle from "react-syntax-highlighter/dist/esm/styles/hljs/arta";

const CodeBlock: React.FC<
  React.JSX.IntrinsicElements["code"] & {
    node?: Element | undefined;
    style: Record<string, React.CSSProperties>;
  }
> = ({ children, className, color: _color, ref: _ref, node: _node, style }) => {
  const match = /language-(\w+)/.exec(className ?? "");
  const textWithOutTrailingNewLine = String(children).replace(/\n$/, "");

  const language: string = match && match.length > 0 ? match[1] : "text";
  return (
    <SyntaxHighlighter
      style={style}
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

export type MarkdownProps = {
  children: string;
  className?: string;
  style?: Record<string, React.CSSProperties>;
};
export const Markdown: React.FC<MarkdownProps> = ({
  children,
  className,
  style = hljsStyle,
}) => {
  return (
    <ReactMarkdown
      className={classNames(styles.markdown, className)}
      components={{
        code(props) {
          return <CodeBlock {...props} style={style} />;
        },
        p(props) {
          return <CodeBlock {...props} style={style} />;
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
};

export type CommandMarkdownProps = Omit<MarkdownProps, "style">;
export const CommandMarkdown: React.FC<CommandMarkdownProps> = (props) => (
  <Markdown {...props} />
);

export type ResultMarkdownProps = Omit<MarkdownProps, "style">;
export const ResultMarkdown: React.FC<ResultMarkdownProps> = (props) => {
  const style = resultStyle;
  return <Markdown {...props} style={style} />;
};
