import React from "react";
import ReactMarkdown from "react-markdown";
import SyntaxHighlighter from "react-syntax-highlighter";
import { Code, Button, Flex, Text } from "@radix-ui/themes";
import { RightButton, RightButtonGroup } from "../Buttons/";
import { ScrollArea } from "../ScrollArea";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
import "./highlightjs.css";
import styles from "./Markdown.module.css";
import { useConfig } from "../../contexts/config-context";

const PreTagWithButtons: React.FC<
  React.PropsWithChildren<{
    onCopyClick: () => void;
    onNewFileClick: () => void;
    onPasteClick: () => void;
    canPaste?: boolean;
  }>
> = ({
  children,
  onCopyClick,
  onNewFileClick,
  onPasteClick,
  canPaste,
  ...props
}) => {
  const config = useConfig();

  return (
    <ScrollArea scrollbars="horizontal">
      <pre {...props}>
        {config.host === "web" ? (
          <RightButton onClick={onCopyClick}>Copy</RightButton>
        ) : (
          <RightButtonGroup direction="column">
            <Flex gap="1" justify="end">
              <Button variant="surface" size="1" onClick={onNewFileClick}>
                New File
              </Button>
              <Button size="1" variant="surface" onClick={onCopyClick}>
                Copy
              </Button>
            </Flex>
            {canPaste && (
              <Button variant="surface" size="1" onClick={onPasteClick}>
                Paste
              </Button>
            )}
          </RightButtonGroup>
        )}
        {children}
      </pre>
    </ScrollArea>
  );
};

const PreTagWithoutButtons: React.FC<React.PropsWithChildren> = (props) => {
  return (
    <ScrollArea scrollbars="horizontal">
      <pre {...props} />
    </ScrollArea>
  );
};

type MarkdownWithControls = {
  onCopyClick?: (str: string) => void;
  onNewFileClick?: (str: string) => void;
  onPasteClick?: (str: string) => void;
  canPaste?: boolean;
};

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children"
> &
  MarkdownWithControls;

export const Markdown: React.FC<MarkdownProps> = ({
  children,
  onCopyClick,
  onNewFileClick,
  onPasteClick,
  canPaste,
}) => {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkBreaks]}
      components={{
        code(props) {
          const {
            children,
            className,
            node,
            color: _color,
            ref: _ref,
            ...rest
          } = props;
          const match = /language-(\w+)/.exec(className ?? "");
          const textWithOutTrailingNewLine = String(children).replace(
            /\n$/,
            "",
          );

          const renderedText = node?.children.reduce((acc, elem) => {
            if (elem.type === "text") {
              return acc + elem.value;
            }
            return acc;
          }, "");

          const PreTag: React.FC<React.PropsWithChildren> = (props) => {
            if (!onCopyClick || !onNewFileClick || !onPasteClick)
              return <PreTagWithoutButtons {...props} />;
            return (
              <PreTagWithButtons
                canPaste={canPaste}
                onCopyClick={() => {
                  if (renderedText) {
                    onCopyClick(renderedText);
                  }
                }}
                onNewFileClick={() => {
                  if (renderedText) {
                    onNewFileClick(renderedText);
                  }
                }}
                onPasteClick={() => {
                  if (renderedText) {
                    onPasteClick(renderedText);
                  }
                }}
                {...props}
              />
            );
          };

          return match ? (
            <Text size="2">
              <SyntaxHighlighter
                className={className}
                PreTag={PreTag}
                language={match[1]}
                useInlineStyles={false}
                // wrapLines={true}
                // wrapLongLines
              >
                {textWithOutTrailingNewLine}
              </SyntaxHighlighter>
            </Text>
          ) : (
            <Code {...rest} className={classNames(styles.code, className)}>
              {children}
            </Code>
          );
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
};
