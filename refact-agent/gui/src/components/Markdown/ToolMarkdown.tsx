import React, { useMemo } from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import { MarkdownCodeBlock, type MarkdownCodeBlockProps } from "./CodeBlock";
import toolStyles from "./ToolMarkdown.module.css";
import "katex/dist/katex.min.css";

export type ToolMarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Pick<MarkdownCodeBlockProps, "style" | "color">;

/**
 * ToolMarkdown - A specialized markdown renderer for tool outputs
 * 
 * Key differences from regular Markdown:
 * - All text renders at consistent size (terminal-like)
 * - Headings are bold but NOT larger (no scaling)
 * - Uses plain HTML elements with CSS styling (no Radix Text)
 * - Designed to match MarkdownCodeBlock visual style exactly
 */
export const ToolMarkdown: React.FC<ToolMarkdownProps> = ({
  children,
  allowedElements,
  unwrapDisallowed,
  style,
  color,
}) => {
  const components: Partial<Components> = useMemo(() => {
    return {
      // Paragraphs
      p({ color: _color, ref: _ref, node: _node, ...props }) {
        return <p className={toolStyles.paragraph} {...props} />;
      },

      // Headings - all same size, just bold
      h1({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },
      h2({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },
      h3({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },
      h4({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },
      h5({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },
      h6({ color: _color, ref: _ref, node: _node, ...props }) {
        return <div className={toolStyles.heading} {...props} />;
      },

      // Lists
      ol(props) {
        return <ol className={toolStyles.list} {...props} />;
      },
      ul(props) {
        return <ul className={toolStyles.list} {...props} />;
      },
      li({ color: _color, ref: _ref, node: _node, ...props }) {
        return <li className={toolStyles.listItem} {...props} />;
      },

      // Code blocks - use the same style as tool output
      code({ style: _style, color: _color, ...props }) {
        return <MarkdownCodeBlock color={color} style={style} {...props} />;
      },

      // Inline elements
      blockquote({ color: _color, ref: _ref, node: _node, ...props }) {
        return <blockquote className={toolStyles.blockquote} {...props} />;
      },
      em({ color: _color, ref: _ref, node: _node, ...props }) {
        return <em {...props} />;
      },
      strong({ color: _color, ref: _ref, node: _node, ...props }) {
        return <strong {...props} />;
      },
      b({ color: _color, ref: _ref, node: _node, ...props }) {
        return <strong {...props} />;
      },
      i({ color: _color, ref: _ref, node: _node, ...props }) {
        return <em {...props} />;
      },
      a({ color: _color, ref: _ref, node: _node, ...props }) {
        const shouldTargetBeBlank =
          props.href &&
          (props.href.startsWith("http") || props.href.startsWith("https"));
        return (
          <a
            className={toolStyles.link}
            {...props}
            target={shouldTargetBeBlank ? "_blank" : undefined}
          />
        );
      },

      // Tables
      table({ color: _color, ref: _ref, node: _node, ...props }) {
        return <table className={toolStyles.table} {...props} />;
      },
      tbody({ color: _color, ref: _ref, node: _node, ...props }) {
        return <tbody {...props} />;
      },
      thead({ color: _color, ref: _ref, node: _node, ...props }) {
        return <thead className={toolStyles.thead} {...props} />;
      },
      tr({ color: _color, ref: _ref, node: _node, ...props }) {
        return <tr {...props} />;
      },
      th({ color: _color, ref: _ref, node: _node, ...props }) {
        return <th className={toolStyles.th} {...props} />;
      },
      td({ color: _color, ref: _ref, node: _node, width: _width, ...props }) {
        return <td className={toolStyles.td} {...props} />;
      },
    };
  }, [style, color]);

  return (
    <ReactMarkdown
      className={toolStyles.root}
      remarkPlugins={[remarkBreaks, remarkMath, remarkGfm]}
      rehypePlugins={[rehypeKatex]}
      allowedElements={allowedElements}
      unwrapDisallowed={unwrapDisallowed}
      components={components}
    >
      {children}
    </ReactMarkdown>
  );
};
