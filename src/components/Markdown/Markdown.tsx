import React, { Key, useCallback, useMemo, useState } from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import {
  MarkdownCodeBlock,
  type MarkdownControls,
  type MarkdownCodeBlockProps,
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
  Box,
} from "@radix-ui/themes";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";
import { diffApi } from "../../services/refact";
import {
  useConfig,
  useDiffApplyMutation,
  useEventsBusForIDE,
} from "../../hooks";
import { selectOpenFiles } from "../../features/OpenFiles/openFilesSlice";
import { useSelector } from "react-redux";
import { ErrorCallout, DiffWarningCallout } from "../Callout";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Partial<MarkdownControls> &
  Pick<
    MarkdownCodeBlockProps,
    "startingLineNumber" | "showLineNumbers" | "useInlineStyles" | "style"
  > & { canHavePins?: boolean };

const MaybePinButton: React.FC<{
  key?: Key | null;
  children?: React.ReactNode;
  getMarkdown: (pin: string) => string | undefined;
}> = ({ children, getMarkdown }) => {
  const { host } = useConfig();

  const { diffPreview } = useEventsBusForIDE();
  const { onSubmit, result: _result } = useDiffApplyMutation();
  const openFiles = useSelector(selectOpenFiles);
  const isPin = typeof children === "string" && children.startsWith("📍");
  const markdown = getMarkdown(String(children));

  const [errorMessage, setErrorMessage] = useState<{
    type: "warning" | "error";
    text: string;
  } | null>(null);

  const [getPatch, _patchResult] =
    diffApi.useLazyPatchSingleFileFromTicketQuery();

  const handleShow = useCallback(() => {
    if (typeof children !== "string") return;
    if (!markdown) return;

    getPatch({ pin: children, markdown })
      .unwrap()
      .then((patch) => {
        if (patch.chunks.length === 0) {
          throw new Error("No Chunks to show");
        }
        diffPreview(patch);
      })
      .catch(() => {
        setErrorMessage({ type: "warning", text: "No patch to show" });
      });
  }, [children, diffPreview, getPatch, markdown]);

  const handleApply = useCallback(() => {
    if (typeof children !== "string") return;
    if (!markdown) return;
    getPatch({ pin: children, markdown })
      .unwrap()
      .then((patch) => {
        const files = patch.results.reduce<string[]>((acc, cur) => {
          const { file_name_add, file_name_delete, file_name_edit } = cur;
          if (file_name_add) acc.push(file_name_add);
          if (file_name_delete) acc.push(file_name_delete);
          if (file_name_edit) acc.push(file_name_edit);
          return acc;
        }, []);

        if (files.length === 0) {
          setErrorMessage({ type: "warning", text: "No chunks to apply" });
          return;
        }

        const fileIsOpen = files.some((file) => openFiles.includes(file));

        if (fileIsOpen) {
          diffPreview(patch);
        } else {
          const chunks = patch.chunks;
          const toApply = chunks.map(() => true);
          void onSubmit({ chunks, toApply });
        }
      })
      .catch((error: Error) => {
        setErrorMessage({
          type: "error",
          text: error.message
            ? "Failed to apply patch: " + error.message
            : "Failed to apply patch.",
        });
      });
  }, [children, diffPreview, getPatch, markdown, onSubmit, openFiles]);

  const handleCalloutClick = useCallback(() => {
    setErrorMessage(null);
  }, []);

  if (isPin) {
    return (
      <Box>
        <Flex my="2" gap="2" wrap="wrap-reverse">
          <Text
            as="p"
            wrap="wrap"
            style={{ lineBreak: "anywhere", wordBreak: "break-all" }}
          >
            {children}
          </Text>
          <Flex gap="2" justify="end" ml="auto">
            {host !== "web" && (
              <Button
                size="1"
                // loading={patchResult.isFetching}
                onClick={handleShow}
                title="Show Patch"
                disabled={!!errorMessage}
              >
                Open
              </Button>
            )}
            <Button
              size="1"
              // loading={patchResult.isFetching}
              onClick={handleApply}
              disabled={!!errorMessage}
              title="Apply patch"
            >
              Apply
            </Button>
          </Flex>
        </Flex>
        {errorMessage && errorMessage.type === "error" && (
          <ErrorCallout onClick={handleCalloutClick} timeout={3000}>
            {errorMessage.text}
          </ErrorCallout>
        )}
        {errorMessage && errorMessage.type === "warning" && (
          <DiffWarningCallout
            timeout={3000}
            onClick={handleCalloutClick}
            message={errorMessage.text}
          />
        )}
      </Box>
    );
  }

  return (
    <Text my="2" as="p">
      {children}
    </Text>
  );
};

function processPinAndMarkdown(message?: string | null): Map<string, string> {
  if (!message) return new Map<string, string>();

  const regexp = /📍[\s\S]*?\n```\n/g;

  const results = message.match(regexp) ?? [];

  const pinsAndMarkdown = results.map<[string, string]>((result) => {
    const firstNewLine = result.indexOf("\n");
    const pin = result.slice(0, firstNewLine);
    const markdown = result.slice(firstNewLine + 1);
    return [pin, markdown];
  });

  const hashMap = new Map(pinsAndMarkdown);

  return hashMap;
}

const _Markdown: React.FC<MarkdownProps> = ({
  children,
  allowedElements,
  unwrapDisallowed,
  canHavePins,
  ...rest
}) => {
  const pinsAndMarkdown = useMemo<Map<string, string>>(
    () => processPinAndMarkdown(children),
    [children],
  );

  const getMarkDownForPin = useCallback(
    (pin: string) => {
      return pinsAndMarkdown.get(pin);
    },
    [pinsAndMarkdown],
  );

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
          return <MaybePinButton {...props} getMarkdown={getMarkDownForPin} />;
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
  }, [getMarkDownForPin, rest, canHavePins]);
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
