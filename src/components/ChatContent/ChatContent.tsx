import React, { useEffect, useImperativeHandle } from "react";
import {
  ChatMessages,
  isChatContextFileMessage,
  isUserMessage,
} from "../../services/refact";
import { Markdown, MarkdownProps } from "../Markdown";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Text } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
// import { SystemInput } from "./SystemInput";

const PlaceHolderText: React.FC = () => (
  <Text>Welcome to Refact chat! How can I assist you today?</Text>
);

function findLastIndexOf<T>(items: T[], predicate: (item: T) => boolean) {
  for (let i = items.length - 1; i >= 0; i--) {
    const item = items[i];
    if (predicate(item)) {
      return i;
    }
  }
  return -1;
}

export type ChatContentProps = {
  messages: ChatMessages;
  onRetry: (question: ChatMessages) => void;
  isWaiting: boolean;
  canPaste: boolean;
  isStreaming: boolean;
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
    } = props;

    const innerRef = React.useRef<HTMLDivElement>(null);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    useImperativeHandle(ref, () => innerRef.current!, []);

    useEffect(() => {
      innerRef.current?.scrollIntoView &&
        innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    }, [messages]);

    const lastUserMessageIndex = findLastIndexOf(messages, isUserMessage);

    return (
      <ScrollArea style={{ flexGrow: 1, height: "auto" }} scrollbars="vertical">
        <Flex direction="column" className={styles.content}>
          {messages.length === 0 && <PlaceHolderText />}
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
                  canRetry={lastUserMessageIndex === index}
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
                >
                  {text}
                </AssistantInput>
              );
              // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
            } else if (role === "system") {
              return null;
              // return <SystemInput key={index}>{text}</SystemInput>;
            } else {
              return <Markdown key={index}>{text}</Markdown>;
            }
          })}
          {isWaiting && <Spinner />}
          <div ref={innerRef} />
        </Flex>
      </ScrollArea>
    );
  },
);

ChatContent.displayName = "ChatContent";
