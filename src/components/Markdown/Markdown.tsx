import React from "react";
import ReactMarkdown from "react-markdown";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import { MarkdownCodeBlock, type MarkdownControls } from "./CodeBlock";
import {
  Text,
  Heading,
  Blockquote,
  Em,
  Kbd,
  Link,
  Quote,
  Strong,
} from "@radix-ui/themes";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children"
> &
  Partial<MarkdownControls>;

export const Markdown: React.FC<MarkdownProps> = ({ children, ...rest }) => {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkBreaks, remarkMath]}
      rehypePlugins={[rehypeKatex]}
      components={{
        ol(props) {
          return (
            <ol
              {...props}
              className={classNames(styles.list, props.className)}
            />
          );
        },
        ul(props) {
          return (
            <ul
              {...props}
              className={classNames(styles.list, props.className)}
            />
          );
        },
        code(props) {
          return <MarkdownCodeBlock {...props} {...rest} />;
        },
        p({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Text as="p" {...props} />;
        },
        h1({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h1" {...props} />;
        },
        h2({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h2" {...props} />;
        },
        h3({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h3" {...props} />;
        },
        h4({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h4" {...props} />;
        },
        h5({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h5" {...props} />;
        },
        h6({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Heading as="h6" {...props} />;
        },
        blockquote({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Blockquote {...props} />;
        },
        em({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Em {...props} />;
        },
        kbd({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Kbd {...props} />;
        },
        a({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Link {...props} />;
        },
        q({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Quote {...props} />;
        },
        strong({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Strong {...props} />;
        },
        b({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Text {...props} weight="bold" />;
        },
        i({ color: _color, ref: _ref, node: _node, ...props }) {
          return <Em {...props} />;
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
};
