import React from "react";
import ReactMarkdown from "react-markdown";
import styles from "./Command.module.css";
import { type SyntaxHighlighterProps } from "react-syntax-highlighter";
import classNames from "classnames";
import type { Element } from "hast";
import hljsStyle from "react-syntax-highlighter/dist/esm/styles/hljs/agate";
import resultStyle from "react-syntax-highlighter/dist/esm/styles/hljs/arta";
import {
  MarkdownCodeBlock,
  type MarkdownCodeBlockProps,
} from "../Markdown/CodeBlock";

type CodeBlockProps = React.JSX.IntrinsicElements["code"] & {
  node?: Element | undefined;
  style?: MarkdownCodeBlockProps["style"];
} & Pick<SyntaxHighlighterProps, "showLineNumbers" | "startingLineNumber">;

export type MarkdownProps = {
  children: string;
  className?: string;
} & Pick<CodeBlockProps, "showLineNumbers" | "startingLineNumber" | "style">;

export const Markdown: React.FC<MarkdownProps> = ({
  children,
  className,
  style = hljsStyle,
}) => {
  return (
    <ReactMarkdown
      className={classNames(styles.markdown, className)}
      components={{
        code({ color: _color, ref: _ref, node: _node, ...props }) {
          return <MarkdownCodeBlock {...props} style={style} />;
        },
        p({ color: _color, ref: _ref, node: _node, ...props }) {
          return <MarkdownCodeBlock {...props} style={style} />;
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
