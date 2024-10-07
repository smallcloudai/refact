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
import { diffApi, isDetailMessage } from "../../services/refact";
import {
  useConfig,
  useDiffApplyMutation,
  useEventsBusForIDE,
} from "../../hooks";
import { selectOpenFiles } from "../../features/OpenFiles/openFilesSlice";
import { useSelector } from "react-redux";
import { ErrorCallout, DiffWarningCallout } from "../Callout";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
} from "../../features/Chat";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Partial<MarkdownControls> &
  Pick<
    MarkdownCodeBlockProps,
    "startingLineNumber" | "showLineNumbers" | "useInlineStyles" | "style"
  > & { canHavePins?: boolean };

const usePinActions = () => {
  const { diffPreview, startFileAnimation, stopFileAnimation } =
    useEventsBusForIDE();
  const { onSubmit, result: _result } = useDiffApplyMutation();
  const openFiles = useSelector(selectOpenFiles);
  const messages = useSelector(selectMessages);
  const isStreaming = useSelector(selectIsStreaming);
  const isWaiting = useSelector(selectIsWaiting);

  const [errorMessage, setErrorMessage] = useState<{
    type: "warning" | "error";
    text: string;
  } | null>(null);

  const resetErrorMessage = useCallback(() => {
    setErrorMessage(null);
  }, []);

  const [getPatch, patchResult] =
    diffApi.useLazyPatchSingleFileFromTicketQuery();

  const disable = useMemo(() => {
    return !!errorMessage || isStreaming || isWaiting || patchResult.isFetching;
  }, [errorMessage, isStreaming, isWaiting, patchResult.isFetching]);

  const handleShow = useCallback(
    (pin: string) => {
      const [, , fileName] = pin.split(" ");
      startFileAnimation(fileName);
      getPatch({ pin, messages })
        .unwrap()
        .then((maybeDetail) => {
          if (isDetailMessage(maybeDetail)) {
            const error = new Error(maybeDetail.detail);
            throw error;
          }
          return maybeDetail;
        })
        .then((patch) => {
          stopFileAnimation(fileName);
          if (patch.chunks.length === 0) {
            setErrorMessage({ type: "warning", text: "No Chunks to show." });
          } else {
            diffPreview(patch);
          }
        })
        .catch((error: Error | { data: { detail: string } }) => {
          stopFileAnimation(fileName);
          if ("message" in error) {
            setErrorMessage({
              type: "error",
              text: "Failed to open patch: " + error.message,
            });
          } else {
            setErrorMessage({
              type: "error",
              text: "Failed to open patch: " + error.data.detail,
            });
          }
        });
    },
    [diffPreview, getPatch, messages, startFileAnimation, stopFileAnimation],
  );

  const handleApply = useCallback(
    (pin: string) => {
      const [, , fileName] = pin.split(" ");
      startFileAnimation(fileName);

      getPatch({ pin, messages })
        .unwrap()
        .then((maybeDetail) => {
          if (isDetailMessage(maybeDetail)) {
            const error = new Error(maybeDetail.detail);
            throw error;
          }
          return maybeDetail;
        })
        .then((patch) => {
          stopFileAnimation(fileName);
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
        .catch((error: Error | { data: { detail: string } }) => {
          stopFileAnimation(fileName);
          if ("message" in error) {
            setErrorMessage({
              type: "error",
              text: "Failed to apply patch: " + error.message,
            });
          } else {
            setErrorMessage({
              type: "error",
              text: "Failed to apply patch: " + error.data.detail,
            });
          }
        });
    },
    [
      diffPreview,
      getPatch,
      messages,
      onSubmit,
      openFiles,
      startFileAnimation,
      stopFileAnimation,
    ],
  );

  return {
    errorMessage,
    handleShow,
    patchResult,
    handleApply,
    resetErrorMessage,
    disable,
  };
};

const MaybePinButton: React.FC<{
  key?: Key | null;
  children?: React.ReactNode;
}> = ({ children }) => {
  const { host } = useConfig();

  const isPin = typeof children === "string" && children.startsWith("📍");

  const { handleApply, handleShow, errorMessage, resetErrorMessage, disable } =
    usePinActions();

  if (isPin) {
    const [cmd, ticket, filePath] = children.split(" ");
    return (
      <Box>
        <Flex my="2" gap="2" wrap="wrap-reverse">
          <Text
            as="p"
            wrap="wrap"
            style={{ lineBreak: "anywhere", wordBreak: "break-all" }}
          >
            {cmd} {ticket}{" "}
            {host !== "web" || import.meta.env.MODE === "development" ? (
              <Link
                wrap="wrap"
                href=""
                // TODO: button that looks like a link
                // disabled={disable}
                onClick={(event) => {
                  event.preventDefault();
                  handleShow(children);
                }}
              >
                {filePath}
              </Link>
            ) : (
              filePath
            )}
          </Text>
          <Flex gap="2" justify="end" ml="auto">
            <Button
              size="1"
              onClick={() => handleApply(children)}
              disabled={disable}
              title="Apply patch"
            >
              Apply
            </Button>
          </Flex>
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
      </Box>
    );
  }

  return (
    <Text my="2" as="p">
      {children}
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
