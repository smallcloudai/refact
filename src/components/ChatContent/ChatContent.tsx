import React, { useCallback } from "react";
import {
  ChatMessages,
  ToolResult,
  isChatContextFileMessage,
  isToolMessage,
} from "../../services/refact";
import type { MarkdownProps } from "../Markdown";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Text, Container, Link } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { MemoryContent } from "./MemoryContent";
import { useAutoScroll } from "./useAutoScroll";
import { PlainText } from "./PlainText";
import { useConfig } from "../../contexts/config-context";

const PlaceHolderText: React.FC<{ onClick: () => void }> = ({ onClick }) => {
  const config = useConfig();
  const hasVecDB = config.features?.vecdb ?? false;
  const hasAst = config.features?.ast ?? false;

  const openSettings = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      onClick();
    },
    [onClick],
  );

  if (config.host === "web") {
    return <Text>Welcome to Refact chat! How can I assist you today?</Text>;
  }

  if (!hasVecDB && !hasAst) {
    return (
      <Text>
        💡 You can turn on VecDB and AST in{" "}
        <Link onClick={openSettings}>settings</Link>.
      </Text>
    );
  } else if (!hasVecDB) {
    return (
      <Text>
        💡 You can turn on VecDB in <Link onClick={openSettings}>settings</Link>
        .
      </Text>
    );
  } else if (!hasAst) {
    return (
      <Text>
        💡 You can turn on AST in <Link onClick={openSettings}>settings</Link>.
      </Text>
    );
  }
  return <Text>Welcome to Refact chat! How can I assist you today?</Text>;
};

export type ChatContentProps = {
  messages: ChatMessages;
  onRetry: (question: ChatMessages) => void;
  isWaiting: boolean;
  canPaste: boolean;
  isStreaming: boolean;
  openSettings: () => void;
} & Pick<MarkdownProps, "onNewFileClick" | "onPasteClick">;

export const ChatContent = React.forwardRef<HTMLDivElement, ChatContentProps>(
  (props, ref) => {
    const {
      messages,
      onRetry,
      isWaiting,
      onNewFileClick,
      onPasteClick,
      canPaste,
      isStreaming,
      openSettings,
    } = props;

    const { innerRef, handleScroll } = useAutoScroll({
      ref,
      messages,
      isStreaming,
    });

    const toolResultsMap = React.useMemo(() => {
      return messages.reduce<Record<string, ToolResult>>((acc, message) => {
        if (!isToolMessage(message)) return acc;
        const result = message[1];
        return {
          ...acc,
          [result.tool_call_id]: result,
        };
      }, {});
    }, [messages]);

    return (
      <ScrollArea
        style={{ flexGrow: 1, height: "auto" }}
        scrollbars="vertical"
        onScroll={handleScroll}
      >
        <Flex direction="column" className={styles.content} p="2" gap="2">
          {messages.length === 0 && <PlaceHolderText onClick={openSettings} />}
          {messages.map((message, index) => {
            if (isChatContextFileMessage(message)) {
              const [, files] = message;
              return <ContextFiles key={index} files={files} />;
            }

            const [role, text] = message;

            if (role === "user") {
              const handleRetry = (question: string) => {
                const toSend = messages
                  .slice(0, index)
                  .concat([["user", question]]);
                onRetry(toSend);
              };
              return (
                <UserInput
                  onRetry={handleRetry}
                  key={index}
                  disableRetry={isStreaming || isWaiting}
                >
                  {text}
                </UserInput>
              );
            } else if (role === "assistant") {
              return (
                <AssistantInput
                  onNewFileClick={onNewFileClick}
                  onPasteClick={onPasteClick}
                  canPaste={canPaste}
                  key={index}
                  message={text}
                  toolCalls={message[2]}
                  toolResults={toolResultsMap}
                />
              );
            } else if (role === "tool") {
              return null;
            } else if (role === "context_memory") {
              return <MemoryContent key={index} items={text} />;
            } else if (role === "plain_text") {
              return <PlainText key={index}>{text}</PlainText>;
            } else {
              return null;
              // return <Markdown key={index}>{text}</Markdown>;
            }
          })}
          {isWaiting && (
            <Container py="4">
              <Spinner />
            </Container>
          )}
          <div ref={innerRef} />
        </Flex>
      </ScrollArea>
    );
  },
);

ChatContent.displayName = "ChatContent";
