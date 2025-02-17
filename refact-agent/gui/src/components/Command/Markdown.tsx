import React from "react";
import ReactMarkdown, {
  defaultUrlTransform,
  type UrlTransform,
} from "react-markdown";
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

const dataUrlPattern =
  /^data:image\/(png|jpeg|gif|bmp|webp);base64,[A-Za-z0-9+/]+={0,2}$/;

const urlTransform: UrlTransform = (value) => {
  if (dataUrlPattern.test(value)) {
    return value;
  }
  return defaultUrlTransform(value);
};

type CodeBlockProps = React.JSX.IntrinsicElements["code"] & {
  node?: Element | undefined;
  style?: MarkdownCodeBlockProps["style"];
} & Pick<SyntaxHighlighterProps, "showLineNumbers" | "startingLineNumber">;

export type MarkdownProps = {
  children: string;
  className?: string;
  isInsideScrollArea?: boolean;
} & Pick<CodeBlockProps, "showLineNumbers" | "startingLineNumber" | "style">;

const Image: React.FC<
  React.DetailedHTMLProps<
    React.ImgHTMLAttributes<HTMLImageElement>,
    HTMLImageElement
  >
> = ({ ...props }) => {
  return <img {...props} className={styles.image} />;
};

export const Markdown: React.FC<MarkdownProps> = ({
  children,
  className,
  isInsideScrollArea,
  style = hljsStyle,
}) => {
  return (
    <ReactMarkdown
      urlTransform={urlTransform}
      className={classNames(styles.markdown, className, {
        [styles.isInsideScrollArea]: isInsideScrollArea,
      })}
      components={{
        code({ color: _color, ref: _ref, node: _node, ...props }) {
          return <MarkdownCodeBlock {...props} style={style} />;
        },
        p({ color: _color, ref: _ref, node: _node, ...props }) {
          return <div {...props} />;
        },

        img({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Image {...props} />;
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
