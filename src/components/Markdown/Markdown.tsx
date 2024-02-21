import React from "react";
import ReactMarkdown from "react-markdown";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
import "./highlightjs.css";
import styles from "./Markdown.module.css";
import { MarkdownCodeBlock, type MarkdownControls } from "./CodeBlock";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children"
> &
  Partial<MarkdownControls>;

export const Markdown: React.FC<MarkdownProps> = ({ children, ...rest }) => {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkBreaks]}
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
      }}
    >
      {children}
    </ReactMarkdown>
  );
};
