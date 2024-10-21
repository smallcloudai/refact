import React, {
  Key,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
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
  Button,
  Flex,
  Card,
} from "@radix-ui/themes";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";
import { usePatchActions } from "../../hooks";

import { ErrorCallout, DiffWarningCallout } from "../Callout";

import { TruncateLeft } from "../Text";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Pick<
    MarkdownCodeBlockProps,
    "startingLineNumber" | "showLineNumbers" | "useInlineStyles" | "style"
  > & { canHavePins?: boolean } & Partial<MarkdownControls>;

const PinMessages: React.FC<{
  children: string;
}> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  const {
    handleShow,
    errorMessage,
    resetErrorMessage,
    disable,
    openFile,
    handlePaste,
    canPaste,
  } = usePatchActions();

  const getMarkdown = useCallback(() => {
    return (
      ref.current?.parentElement?.nextElementSibling?.querySelector("code")
        ?.textContent ?? null
    );
  }, []);

  const onDiffClick = useCallback(() => {
    const markdown = getMarkdown();
    if (markdown) {
      handlePaste(markdown);
    }
  }, [getMarkdown, handlePaste]);

  const [hasMarkdown, setHasMarkdown] = useState<boolean>(false);

  useEffect(() => {
    if (!ref.current) {
      setHasMarkdown(false);
    } else {
      const markdown = !!getMarkdown();
      setHasMarkdown(markdown);
    }
  }, [getMarkdown]);

  if (children.startsWith("📍OTHER")) {
    return null;
  }

  const [_cmd, _ticket, filePath, ..._rest] = children.split(" ");
  return (
    <Card
      className={styles.patch_title}
      size="1"
      variant="surface"
      mt="4"
      ref={ref}
    >
      <Flex gap="2" py="2" pl="2" justify="between">
        <TruncateLeft>
          <Link
            title="Open file"
            onClick={(event) => {
              event.preventDefault();
              openFile({ file_name: filePath });
            }}
          >
            {filePath}
          </Link>
        </TruncateLeft>{" "}
        <div style={{ flexGrow: 1 }} />
        <Button
          size="1"
          onClick={() => handleShow(children)}
          disabled={disable}
          title={`Show: ${children}`}
        >
          ➕ Auto Apply
        </Button>
        <Button
          size="1"
          onClick={onDiffClick}
          disabled={disable || !hasMarkdown || !canPaste}
          title="Replace the current selection in the ide."
        >
          ➕ Replace Selection
        </Button>
      </Flex>
      {errorMessage && errorMessage.type === "error" && (
        <ErrorCallout onClick={resetErrorMessage} timeout={5000}>
          {errorMessage.text}
        </ErrorCallout>
      )}
      {errorMessage && errorMessage.type === "warning" && (
        <DiffWarningCallout
          timeout={5000}
          onClick={resetErrorMessage}
          message={errorMessage.text}
        />
      )}
    </Card>
  );
};

const MaybePinButton: React.FC<{
  key?: Key | null;
  children?: React.ReactNode;
}> = ({ children }) => {
  const processed = React.Children.map(children, (child, index) => {
    if (typeof child === "string" && child.startsWith("📍")) {
      const key = `pin-message-${index}`;
      return <PinMessages key={key}>{child}</PinMessages>;
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
  canHavePins,
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
      code({ style: _style, ...props }) {
        return <MarkdownCodeBlock {...props} {...rest} />;
      },
      p({ color: _color, ref: _ref, node: _node, ...props }) {
        if (canHavePins) {
          return <MaybePinButton {...props} />;
        }
        return <Text my="2" as="p" {...props} />;
      },
      h1({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="6" size="8" as="h1" {...props} />;
      },
      h2({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="6" size="7" as="h2" {...props} />;
      },
      h3({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="6" size="6" as="h3" {...props} />;
      },
      h4({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="5" size="5" as="h4" {...props} />;
      },
      h5({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="4" size="4" as="h5" {...props} />;
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
    };
  }, [rest, canHavePins]);
  return (
    <ReactMarkdown
      className={styles.markdown}
      remarkPlugins={[remarkBreaks, remarkMath]}
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
