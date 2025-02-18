import React, { Key, useMemo } from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import {
  MarkdownCodeBlock,
  type MarkdownCodeBlockProps,
  type MarkdownControls,
} from "./CodeBlock";
import {
  Text,
  Heading,
  Blockquote,
  Em,
  Kbd,
  Link,
  Quote,
  Strong,
  Flex,
  Table,
} from "@radix-ui/themes";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import remarkGfm from "remark-gfm";
import "katex/dist/katex.min.css";
import { useLinksFromLsp } from "../../hooks";

import { ChatLinkButton } from "../ChatLinks";
import { extractLinkFromPuzzle } from "../../utils/extractLinkFromPuzzle";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Pick<
    MarkdownCodeBlockProps,
    | "startingLineNumber"
    | "showLineNumbers"
    | "useInlineStyles"
    | "style"
    | "color"
  > & {
    canHaveInteractiveElements?: boolean;
    wrap?: boolean;
  } & Partial<MarkdownControls>;

const PuzzleLink: React.FC<{
  children: string;
}> = ({ children }) => {
  const { handleLinkAction } = useLinksFromLsp();
  const link = extractLinkFromPuzzle(children);

  if (!link) return children;

  return (
    <Flex direction="column" align="start" gap="2" mt="2">
      <ChatLinkButton link={link} onClick={handleLinkAction} />
    </Flex>
  );
};

const MaybeInteractiveElement: React.FC<{
  key?: Key | null;
  children?: React.ReactNode;
}> = ({ children }) => {
  const processed = React.Children.map(children, (child, index) => {
    if (typeof child === "string" && child.startsWith("🧩")) {
      const key = `puzzle-link-${index}`;
      return <PuzzleLink key={key}>{child}</PuzzleLink>;
    }
    return child;
  });

  return (
    <Text className={styles.maybe_pin} my="2">
      {processed}
    </Text>
  );
};

const _Markdown: React.FC<MarkdownProps> = ({
  children,
  allowedElements,
  unwrapDisallowed,
  canHaveInteractiveElements,
  color,
  ...rest
}) => {
  const components: Partial<Components> = useMemo(() => {
    return {
      ol(props) {
        return (
          <ol {...props} className={classNames(styles.list, props.className)} />
        );
      },
      ul(props) {
        return (
          <ul {...props} className={classNames(styles.list, props.className)} />
        );
      },
      code({ style: _style, color: _color, ...props }) {
        return <MarkdownCodeBlock color={color} {...props} {...rest} />;
      },
      p({ color: _color, ref: _ref, node: _node, ...props }) {
        if (canHaveInteractiveElements) {
          return <MaybeInteractiveElement {...props} />;
        }
        return <Text as="p" {...props} />;
      },
      h1({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="4" size="4" as="h1" {...props} />;
      },
      h2({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h2" {...props} />;
      },
      h3({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h3" {...props} />;
      },
      h4({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h4" {...props} />;
      },
      h5({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h5" {...props} />;
      },
      h6({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h6" {...props} />;
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
        const shouldTargetBeBlank =
          props.href &&
          (props.href.startsWith("http") || props.href.startsWith("https"));
        return (
          <Link
            {...props}
            target={shouldTargetBeBlank ? "_blank" : undefined}
          />
        );
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
      table({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Root my="2" variant="surface" {...props} />;
      },
      tbody({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Body {...props} />;
      },
      thead({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Header {...props} />;
      },
      tr({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Row {...props} />;
      },
      th({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.ColumnHeaderCell {...props} />;
      },
      td({ color: _color, ref: _ref, node: _node, width: _width, ...props }) {
        return <Table.Cell {...props} />;
      },
    };
  }, [rest, canHaveInteractiveElements, color]);
  return (
    <ReactMarkdown
      className={styles.markdown}
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

export const Markdown = React.memo(_Markdown);
